// @ts-expect-error: Missing module declaration
import geographicLib from "./wasm/geographiclib.mjs";
import init from "coursepointer-wasm";

/**
 * Initialize the WASM modules.
 */
export async function initialize() {
  /* eslint-disable */
  (window as any).GEO = await geographicLib();
  /* eslint-enable */

  await init();
}
