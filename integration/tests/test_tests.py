"""Who tests the tests?

Makes sure the integration test library itself is correct.

"""

from pathlib import Path

import pytest

from integration import validate_fit_file
from integration.fixtures import data


def test_validate(data: Path):
    validate_fit_file(data / "cptr001.fit")
    with pytest.raises(ValueError):
        validate_fit_file(data / "invalid_truncated.fit")
