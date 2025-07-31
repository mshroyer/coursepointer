import { initialize } from "./wasm-deps.ts";
import { convert_gpx_to_fit_bytes } from "coursepointer-wasm";

async function initWorker() {
  console.log("Worker started");

  await initialize();

  console.log("Worker initialized");

  // const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
  // await sleep(15000);
  // console.log("Worker woke up");

  onmessage = (e) => {
    console.log("Worker: message received from main script");
    if (e.data.type === "convert_gpx_to_fit") {
      const buf = e.data["buf"];
      const course = new Uint8Array(buf);
      const info = convert_gpx_to_fit_bytes(course);
      self.postMessage({ type: "convert_gpx_to_fit", info: info });
    }
  };

  self.postMessage({ type: "ready" });
}

initWorker();
