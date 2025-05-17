"""Test the main coursepointer-cli binary"""


from integration.cargo import RustBinFunc
from integration.fixtures import cargo, coursepointer_cli


def test_help(coursepointer_cli: RustBinFunc):
    assert "Print help" in coursepointer_cli("--help")
