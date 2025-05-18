"""Who tests the tests?

Makes sure the integration test library itself is correct.

"""

from pathlib import Path

import pytest

from integration import garmin_sdk_read_fit
from integration.fixtures import data


def test_validate(data: Path):
    garmin_sdk_read_fit(data / "cptr001.fit")
    with pytest.raises(ValueError):
        garmin_sdk_read_fit(data / "invalid_truncated.fit")
