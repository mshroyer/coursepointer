from pathlib import Path
import pytest


@pytest.fixture
def data() -> Path:
    # The data directory gets built into the wheel by hatchling by default.
    return Path(__file__).parent / "data"
