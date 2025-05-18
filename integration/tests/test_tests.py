"""Who tests the tests?

Makes sure the integration test library itself is correct.

"""

from pathlib import Path

import pytest

from integration import read_fit_messages
from integration.fixtures import data


def test_validate(data: Path):
    read_fit_messages(data / "cptr001.fit")
    with pytest.raises(ValueError):
        read_fit_messages(data / "invalid_truncated.fit")
