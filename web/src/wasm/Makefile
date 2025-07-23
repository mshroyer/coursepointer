.PHONY: all clean

all: geographiclib.mjs

geographiclib.mjs: ../../../geographiclib/src/DST.cpp ../../../geographiclib/src/EllipticFunction.cpp ../../../geographiclib/src/Geocentric.cpp ../../../geographiclib/src/Geodesic.cpp ../../../geographiclib/src/GeodesicExact.cpp ../../../geographiclib/src/GeodesicLine.cpp ../../../geographiclib/src/GeodesicLineExact.cpp ../../../geographiclib/src/Gnomonic.cpp ../../../geographiclib/src/Math.cpp ../../../src/shim.cpp ../../../src/shim_embind.cpp
#	em++ -O3 -std=c++11 -sEXPORTED_RUNTIME_METHODS=[\"wasmMemory\",\"HEAPU8\",\"UTF8ToString\"] -sEXPORTED_FUNCTIONS=[\"_geodesic_inverse_with_azimuth\",\"_geodesic_direct\",\"_gnomonic_forward\",\"_gnomonic_reverse\",\"_geocentric_forward\",\"_geographiclib_version\",\"_compiler_version\"] -I../../../include -I../../../geographiclib/include $^ --no-entry --bind -o geographiclib.mjs
	em++ -O3 -std=c++11 -sEXPORTED_RUNTIME_METHODS=[\"wasmMemory\",\"HEAPU8\",\"UTF8ToString\"] -sLINKABLE=1 -I../../../include -I../../../geographiclib/include $^ --no-entry --bind -o $@

geographiclib.o: ../../../geographiclib/src/DST.cpp ../../../geographiclib/src/EllipticFunction.cpp ../../../geographiclib/src/Geocentric.cpp ../../../geographiclib/src/Geodesic.cpp ../../../geographiclib/src/GeodesicExact.cpp ../../../geographiclib/src/GeodesicLine.cpp ../../../geographiclib/src/GeodesicLineExact.cpp ../../../geographiclib/src/Gnomonic.cpp ../../../geographiclib/src/Math.cpp ../../../src/shim.cpp
	em++ -O3 -std=c++11 -sLINKABLE=1 -I../../../include -I../../../geographiclib/include $^ --no-entry -c

clean:
	rm -f geographiclib.mjs geographiclib.wasm geographiclib.o
