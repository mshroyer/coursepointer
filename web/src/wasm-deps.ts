// @ts-expect-error: Missing module declaration
import geographicLib from "./wasm/geographiclib.mjs";
import init from "coursepointer-wasm";

/**
 * Initialize the WASM modules.
 */
export async function initialize() {
  // So that I don't have to figure out how to address Vite modules from
  // wasm-bindgen, the Rust code accesses GeographicLib exports as functions
  // attached to the global "window" object.
  //
  // In web workers there is no window object, so we instead create a fake
  // "window" attached to the worker's global self.

  const geo = await geographicLib();
  if (typeof window !== "undefined") {
    (window as any).GEO = geo;
  } else {
    (self as any).window = self;
    (self as any).GEO = geo;
  }
  await init();
}
