from __future__ import annotations


class ObsError(Exception):
    def __init__(
        self,
        code: str,
        message: str,
        status: int,
        request_id: str = "",
        host_id: str = "",
        client_request_id: str = "",
    ) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.status = status
        self.request_id = request_id
        self.host_id = host_id
        self.client_request_id = client_request_id

    def __str__(self) -> str:
        return f"{self.code}: {self.message} (request_id={self.request_id})"


class ClientError(ObsError):
    pass


class ServerError(ObsError):
    pass


class NoSuchKey(ClientError):
    pass


class NoSuchBucket(ClientError):
    pass


class AccessDenied(ClientError):
    pass


_CODE_TO_CLASS: dict[str, type[ObsError]] = {
    "NoSuchKey": NoSuchKey,
    "NoSuchBucket": NoSuchBucket,
    "AccessDenied": AccessDenied,
}


def make_error(
    code: str,
    message: str,
    status: int,
    request_id: str = "",
    host_id: str = "",
    client_request_id: str = "",
) -> ObsError:
    cls = _CODE_TO_CLASS.get(code)
    if cls is None:
        cls = ClientError if status < 500 else ServerError
    return cls(
        code=code,
        message=message,
        status=status,
        request_id=request_id,
        host_id=host_id,
        client_request_id=client_request_id,
    )
