@echo off

cargo build -r
zip -j coursepointer-windows.zip target\release\coursepointer.exe docs\third_party_licenses.md
python3 scripts\release.py upload coursepointer-macos.zip
