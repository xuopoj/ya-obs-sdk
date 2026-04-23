from __future__ import annotations
import logging
import ssl
import uuid
from typing import Callable, Iterator, Union

import httpx

VerifyTypes = Union[bool, str, ssl.SSLContext]

from ._errors import make_error
from ._models import Request, Timeout, RetryPolicy, RetryEvent
from ._retry import RetryLoop
from ._xml import parse_error_response

logger = logging.getLogger("ya_obs")


class RawResponse:
    def __init__(
        self,
        response: httpx.Response,
        client_request_id: str,
        body: bytes | None = None,
    ) -> None:
        self._response = response
        self._body = body
        self.client_request_id = client_request_id
        self.status_code = response.status_code
        self.headers = dict(response.headers)

    def iter_bytes(self, chunk_size: int = 65536) -> Iterator[bytes]:
        try:
            yield from self._response.iter_bytes(chunk_size=chunk_size)
        finally:
            self._response.close()

    def read(self) -> bytes:
        if self._body is not None:
            return self._body
        try:
            return self._response.read()
        finally:
            self._response.close()


class HttpClient:
    def __init__(
        self,
        signer,
        timeout: Timeout,
        retry_policy: RetryPolicy,
        on_retry: Callable[[RetryEvent], None] | None = None,
        verify: VerifyTypes = True,
    ) -> None:
        self._signer = signer
        self._timeout = timeout
        self._retry_policy = retry_policy
        self._on_retry = on_retry
        self._client = httpx.Client(
            timeout=httpx.Timeout(
                connect=timeout.connect,
                read=timeout.read,
                write=timeout.write,
                pool=None,
            ),
            verify=verify,
        )

    def send(self, request: Request, *, stream: bool = False, **overrides) -> RawResponse:
        client_request_id = str(uuid.uuid4())
        retry_loop = RetryLoop(
            policy=overrides.get("retry_policy", self._retry_policy),
            on_retry=self._on_retry,
        )

        def _attempt() -> RawResponse:
            req = Request(
                method=request.method,
                url=request.url,
                headers=dict(request.headers),
                params=request.params,
                body=request.body,
            )
            req.headers["x-ya-obs-client-id"] = client_request_id
            logger.debug("ya_obs: %s %s client_id=%s", req.method, req.url, client_request_id)
            signed = self._signer.sign(req)
            httpx_req = self._client.build_request(
                method=signed.method,
                url=signed.url,
                headers=signed.headers,
                params=signed.params,
                content=signed.body,
            )
            resp = self._client.send(httpx_req, stream=True)
            request_id = resp.headers.get("x-obs-request-id", "")
            if resp.status_code >= 400:
                try:
                    body = resp.read()
                finally:
                    resp.close()
                try:
                    parsed = parse_error_response(body.decode("utf-8"))
                    err = make_error(
                        code=parsed["code"],
                        message=parsed["message"],
                        status=resp.status_code,
                        request_id=parsed.get("request_id", request_id),
                        host_id=parsed.get("host_id", ""),
                        client_request_id=client_request_id,
                    )
                except Exception:
                    err = make_error(
                        code="Unknown",
                        message=body.decode("utf-8", errors="replace"),
                        status=resp.status_code,
                        request_id=request_id,
                        client_request_id=client_request_id,
                    )
                raise err
            if not stream:
                try:
                    body = resp.read()
                finally:
                    resp.close()
                return RawResponse(resp, client_request_id=client_request_id, body=body)
            return RawResponse(resp, client_request_id=client_request_id)

        return retry_loop.run(_attempt)

    def close(self) -> None:
        self._client.close()

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


class AsyncStreamingBody:
    def __init__(self, response: httpx.Response, client_request_id: str) -> None:
        self._response = response
        self.client_request_id = client_request_id
        self._consumed = False

    def _check(self) -> None:
        if self._consumed:
            raise RuntimeError("AsyncStreamingBody already consumed")
        self._consumed = True

    async def read(self) -> bytes:
        self._check()
        try:
            return await self._response.aread()
        finally:
            await self._response.aclose()

    async def iter_bytes(self, chunk_size: int = 65536):
        self._check()
        try:
            async for chunk in self._response.aiter_bytes(chunk_size):
                yield chunk
        finally:
            await self._response.aclose()


class AsyncRawResponse:
    def __init__(
        self,
        response: httpx.Response,
        client_request_id: str,
        body: bytes | None = None,
    ) -> None:
        self._response = response
        self._body = body
        self.client_request_id = client_request_id
        self.status_code = response.status_code
        self.headers = dict(response.headers)

    def stream(self) -> AsyncStreamingBody:
        return AsyncStreamingBody(self._response, self.client_request_id)

    async def read(self) -> bytes:
        if self._body is not None:
            return self._body
        try:
            return await self._response.aread()
        finally:
            await self._response.aclose()


class AsyncHttpClient:
    def __init__(
        self,
        signer,
        timeout: Timeout,
        retry_policy: RetryPolicy,
        on_retry=None,
        verify: VerifyTypes = True,
    ) -> None:
        self._signer = signer
        self._timeout = timeout
        self._retry_policy = retry_policy
        self._on_retry = on_retry
        self._client = httpx.AsyncClient(
            timeout=httpx.Timeout(
                connect=timeout.connect,
                read=timeout.read,
                write=timeout.write,
                pool=None,
            ),
            verify=verify,
        )

    async def send(self, request: Request, *, stream: bool = False, **overrides) -> AsyncRawResponse:
        client_request_id = str(uuid.uuid4())
        req = Request(
            method=request.method,
            url=request.url,
            headers=dict(request.headers),
            params=request.params,
            body=request.body,
        )
        req.headers["x-ya-obs-client-id"] = client_request_id
        signed = self._signer.sign(req)
        httpx_req = self._client.build_request(
            method=signed.method,
            url=signed.url,
            headers=signed.headers,
            params=signed.params,
            content=signed.body,
        )
        resp = await self._client.send(httpx_req, stream=True)
        request_id = resp.headers.get("x-obs-request-id", "")
        if resp.status_code >= 400:
            try:
                body = await resp.aread()
            finally:
                await resp.aclose()
            try:
                parsed = parse_error_response(body.decode("utf-8"))
                err = make_error(
                    code=parsed["code"],
                    message=parsed["message"],
                    status=resp.status_code,
                    request_id=parsed.get("request_id", request_id),
                    host_id=parsed.get("host_id", ""),
                    client_request_id=client_request_id,
                )
            except Exception:
                err = make_error(
                    code="Unknown",
                    message=body.decode("utf-8", errors="replace"),
                    status=resp.status_code,
                    request_id=request_id,
                    client_request_id=client_request_id,
                )
            raise err
        if not stream:
            try:
                body = await resp.aread()
            finally:
                await resp.aclose()
            return AsyncRawResponse(resp, client_request_id=client_request_id, body=body)
        return AsyncRawResponse(resp, client_request_id=client_request_id)

    async def close(self) -> None:
        await self._client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, *args):
        await self.close()
