"""Who tests the tests?

Makes sure the integration test library itself is correct.

"""

from pathlib import Path

import pytest

from integration import garmin_read_messages


def test_validate(data: Path):
    garmin_read_messages(data / "cptr001.fit")
    with pytest.raises(ValueError):
        garmin_read_messages(data / "invalid_truncated.fit")
