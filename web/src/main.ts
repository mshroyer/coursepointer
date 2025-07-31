import "./style.css";
import { initialize } from "./wasm-deps.ts";
import { JsConversionInfo } from "coursepointer-wasm";

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <header>
    <img alt="CoursePointer icon" src="/coursepointer.svg" /><b>CoursePointer</b><span id="elevatorpitch"> – Convert waypoints to FIT course points</span>
  </header>
  <main>
    <div id="explainer">
      <div>
        <p>Choose a GPX file containing exactly one route or track. This will generate a corresponding Garmin FIT course 
        file, in which any waypoints located along the route have been converted to FIT course points for navigation
        with <a href="https://support.garmin.com/en-US/?faq=lQMibRoY2I5Y4pP8EXgxv7">Up Ahead</a>.</p>
        <p>See the <a href="https://github.com/mshroyer/coursepointer/blob/main/README.md">README</a>
        for more about what this does and why.</p>
      </div>
    </div>
  </main>
  <aside>
    <input id="picker" type="file" />
    <p>CoursePointer runs in your browser using WebAssembly. It does not upload your course anywhere.</p>
    <p>This is an alpha web version of an existing <a href="https://github.com/mshroyer/coursepointer/">command-line tool</a>.
    This currently lacks some features present in the command line version, including specifying conversion options.</p>
  </aside>
  <footer>
    <p>© 2025 Mark Shroyer <a href="https://github.com/mshroyer/coursepointer/blob/main/docs/third_party_licenses.md">and others</a>.
     Source code on <a href="https://github.com/mshroyer/coursepointer">GitHub</a>.</p>
  </footer>
`;

/**
 * An object that can either resolve or reject an outstanding Promise.
 */
class Resolver<T> {
  resolve: (result: T) => void;
  reject: (reason?: any) => void;

  constructor(resolve: (result: T) => void, reject: (reason?: any) => void) {
    this.resolve = resolve;
    this.reject = reject;
  }
}

class CoursePointerWorker {
  _worker: Worker;
  _ready: Promise<void>;
  _readyResolver!: Resolver<void>;
  _convertGpxToFitResolvers: Resolver<JsConversionInfo>[];

  constructor() {
    this._ready = new Promise((resolve, reject) => {
      this._readyResolver = new Resolver(resolve, reject);
    });
    this._worker = new Worker(new URL("./worker.ts", import.meta.url), {
      type: "module",
    });
    this._worker.onmessage = (e) => this.handleMessage(e);
    this._convertGpxToFitResolvers = [];
  }

  handleMessage(e: MessageEvent<any>) {
    console.log("Main: got message from worker");
    console.log(e);
    if (e.data.type === "ready") {
      console.log("Main: Got message that worker is ready");
      this._readyResolver.resolve();
    } else if (e.data.type === "convert_gpx_to_fit") {
      const resolver = this._convertGpxToFitResolvers.shift();
      if (e.data.error) {
        resolver!.reject(e.data.error);
      } else {
        resolver!.resolve(e.data.info);
      }
    }
  }

  async convertGpxToFit(buf: ArrayBuffer): Promise<JsConversionInfo> {
    console.log("convertGpxToFit called");
    await this._ready;
    return new Promise((resolve, reject) => {
      this._convertGpxToFitResolvers.push(new Resolver(resolve, reject));
      this._worker.postMessage({ type: "convert_gpx_to_fit", buf: buf });
    });
  }
}

const w = new CoursePointerWorker();

function setupPicker(p: HTMLInputElement) {
  p.addEventListener("change", async (e) => {
    const target = e.target as HTMLInputElement;
    const file: File | undefined = target.files?.[0];
    if (!file) return;

    document.querySelector<HTMLElement>("main")!.innerHTML = `
  <pre id="report"></pre>
  <div id="download"></div>
`;

    const buf = await file.arrayBuffer();
    console.time("convert_gpx_to_fit_bytes");
    const report = document.querySelector<HTMLInputElement>("#report")!;
    let info;
    try {
      info = await w.convertGpxToFit(buf);
      console.log("Main: Got convertGpxToFit result");
      console.log(info);
    } catch (e) {
      report.innerText = `Error converting that file:
${e}

Ensure it's a valid GPX file containing exactly one route or track.
`;
      return;
    } finally {
      console.timeEnd("convert_gpx_to_fit_bytes");
    }
    report.innerText = info.report;
    const blob = new Blob([info.fit_bytes], {
      type: "application/octet-stream",
    });

    document.querySelector<HTMLDivElement>("#download")!.innerHTML = `
  <button id="downloadbtn" type="button">Save as FIT</button>
`;
    document
      .querySelector<HTMLButtonElement>("#downloadbtn")!
      .addEventListener("click", () => {
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = "output.fit";
        a.click();
        URL.revokeObjectURL(url);
      });
  });
}

setupPicker(document.querySelector<HTMLInputElement>("#picker")!);

await initialize();
