import { initialize } from "./wasm-deps.ts";
import { convert_gpx_to_fit_bytes, enumerate_sports } from "coursepointer-wasm";
import {
  ConvertGpxToFitRequest,
  ConvertGpxToFitResponse,
  ReadyResponse,
  WorkerRequest,
} from "./messages.ts";

async function initWorker() {
  console.log("Worker started");

  await initialize();

  console.log("Worker initialized");

  onmessage = (e) => {
    console.log("Worker: message received from main script");
    WorkerRequest.setPrototype(e.data);
    if (e.data instanceof ConvertGpxToFitRequest) {
      const course = e.data.course;
      const options = e.data.options;
      try {
        const info = convert_gpx_to_fit_bytes(course, options);
        postMessage(new ConvertGpxToFitResponse(info, undefined));
      } catch (e) {
        let ex;
        if (e instanceof Error) {
          ex = e;
        } else {
          ex = new Error("Non-error value caught", { cause: e });
        }
        postMessage(new ConvertGpxToFitResponse(undefined, ex));
      }
    } else {
      console.warn("Worker: got unknown WorkerRequest:");
      console.log(e.data);
    }
  };

  postMessage(new ReadyResponse(enumerate_sports()));
}

initWorker();
