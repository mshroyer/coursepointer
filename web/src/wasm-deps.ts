// @ts-expect-error: Missing module declaration
import geographicLib from "./wasm/geographiclib.mjs";
import init from "coursepointer-wasm";

/**
 * Initialize the WASM modules.
 */
export async function initialize() {
  const GEO = await geographicLib();

  /* eslint-disable */
  (window as any).GEO = GEO;
  /* eslint-enable */

  await init();
}
