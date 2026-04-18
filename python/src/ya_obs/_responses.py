from __future__ import annotations
from dataclasses import dataclass, field
from datetime import datetime
from ._streaming import StreamingBody


@dataclass
class PutObjectResponse:
    etag: str
    version_id: str | None
    request_id: str
    client_request_id: str


@dataclass
class GetObjectResponse:
    body: object  # StreamingBody (sync) or async adapter
    content_type: str | None
    content_length: int | None
    etag: str
    last_modified: datetime
    metadata: dict[str, str]
    cache_control: str | None
    content_encoding: str | None
    content_disposition: str | None
    request_id: str
    client_request_id: str


@dataclass
class HeadObjectResponse:
    content_type: str | None
    content_length: int | None
    etag: str
    last_modified: datetime
    metadata: dict[str, str]
    cache_control: str | None
    content_encoding: str | None
    content_disposition: str | None
    request_id: str
    client_request_id: str


@dataclass
class DeleteObjectResponse:
    request_id: str
    client_request_id: str


@dataclass
class ObjectInfo:
    key: str
    etag: str
    size: int
    last_modified: datetime


@dataclass
class ListObjectsPage:
    objects: list[ObjectInfo]
    is_truncated: bool
    next_marker: str | None
    request_id: str
    client_request_id: str


@dataclass
class CopyObjectResponse:
    etag: str
    last_modified: datetime
    request_id: str
    client_request_id: str


@dataclass
class InitiateMultipartResponse:
    upload_id: str
    bucket: str
    key: str
    request_id: str
    client_request_id: str


@dataclass
class UploadPartResponse:
    part_number: int
    etag: str
    request_id: str
    client_request_id: str


@dataclass
class CompleteMultipartResponse:
    etag: str
    location: str
    bucket: str
    key: str
    request_id: str
    client_request_id: str
