import pathlib
import json

VECTORS_DIR = pathlib.Path(__file__).parent.parent.parent / "test-vectors"


def load_vector(category: str, name: str) -> dict:
    return json.loads((VECTORS_DIR / category / f"{name}.json").read_text(encoding="utf-8"))
