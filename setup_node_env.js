import geographicLib from './web/src/wasm/geographiclib.mjs';

async function setupTestEnvironment() {
    global.window = global.window || {};
    global.window.GEO = {};

    try {
        global.window.GEO = await geographicLib();
        console.log("GeographicLib WASM module loaded successfully");
    } catch (error) {
        console.error("Failed to load GeographicLib WASM module:", error);
        throw error;
    }
}

// Run setup immediately
await setupTestEnvironment();
