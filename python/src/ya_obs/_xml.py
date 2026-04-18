from __future__ import annotations
import xml.etree.ElementTree as ET
from datetime import datetime, timezone


_NS = "http://obs.myhwclouds.com/doc/2015-06-30/"


def _tag(name: str) -> str:
    return f"{{{_NS}}}{name}"


def _text(el: ET.Element, tag: str, default: str = "") -> str:
    child = el.find(_tag(tag))
    return child.text or default if child is not None else default


def parse_list_objects(xml: str) -> dict:
    root = ET.fromstring(xml)
    objects = []
    for content in root.findall(_tag("Contents")):
        last_modified_str = _text(content, "LastModified")
        last_modified = datetime.fromisoformat(
            last_modified_str.replace("Z", "+00:00")
        )
        objects.append({
            "key": _text(content, "Key"),
            "etag": _text(content, "ETag"),
            "size": int(_text(content, "Size", "0")),
            "last_modified": last_modified.isoformat(),
        })
    next_marker_el = root.find(_tag("NextMarker"))
    return {
        "name": _text(root, "Name"),
        "is_truncated": _text(root, "IsTruncated").lower() == "true",
        "next_marker": next_marker_el.text if next_marker_el is not None and next_marker_el.text else None,
        "objects": objects,
    }


def parse_initiate_multipart(xml: str) -> dict:
    root = ET.fromstring(xml)
    return {
        "bucket": _text(root, "Bucket"),
        "key": _text(root, "Key"),
        "upload_id": _text(root, "UploadId"),
    }


def parse_error_response(xml: str) -> dict:
    root = ET.fromstring(xml)
    def t(name: str) -> str:
        el = root.find(name)
        return el.text or "" if el is not None else ""
    return {
        "code": t("Code"),
        "message": t("Message"),
        "request_id": t("RequestId"),
        "host_id": t("HostId"),
    }


def serialize_complete_multipart(parts: list[tuple[int, str]]) -> str:
    root = ET.Element("CompleteMultipartUpload")
    for part_number, etag in parts:
        part_el = ET.SubElement(root, "Part")
        pn_el = ET.SubElement(part_el, "PartNumber")
        pn_el.text = str(part_number)
        etag_el = ET.SubElement(part_el, "ETag")
        etag_el.text = etag
    return ET.tostring(root, encoding="unicode")
