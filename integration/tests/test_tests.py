"""Who tests the tests?

Makes sure the integration test library itself is correct.

"""

import pytest

from integration import validate_fit_file
from integration.fixtures import data


def test_validate(data):
    validate_fit_file(data / "cptr001.fit")
    with pytest.raises(ValueError):
        validate_fit_file(data / "invalid_truncated.fit")
