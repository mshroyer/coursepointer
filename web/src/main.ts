import "./style.css";
// @ts-expect-error: Missing module declaration
import geographicLib from "./wasm/geographiclib.mjs";
import init, {
  demo_course_set,
  direct_lon,
  convert_gpx_to_fit_bytes,
} from "coursepointer-wasm";

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <header>
    <p><b>CoursePointer</b> — Convert GPX waypoints to FIT course points</p>
  </header>
  <main>
    <div id="explainer">
      <div>
        <p>Choose a GPX file containing exactly one route or track. This will generate a corresponding Garmin FIT course 
        file, in which any waypoints located along the route have been converted to FIT course points.</p>
        <p>See the <a href="https://github.com/mshroyer/coursepointer/blob/main/README.md">README</a>
        for more about what this does, and why.</p>
      </div>
    </div>
  </main>
  <aside>
    <input id="picker" type="file" />
    <p>CoursePointer runs in your browser using WebAssembly. It does not upload your course anywhere.</p>
  </aside>
  <footer>
    <p>© 2025 Mark Shroyer, <a href="https://github.com/mshroyer/coursepointer/blob/main/docs/third_party_licenses.md">MIT licensed</a>.
     Source code is on <a href="https://github.com/mshroyer/coursepointer">GitHub</a>.</p>
  </footer>
`;

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
    console.time("read_gpx_bytes");
    const course = new Uint8Array(buf);
    const info = convert_gpx_to_fit_bytes(course);
    console.timeEnd("read_gpx_bytes");
    document.querySelector<HTMLPreElement>("#report")!.innerHTML = info.report;
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

/* eslint-disable */

const GEO = await geographicLib();
(window as any).GEO = GEO;
await init();

// Functions exported by coursepointer WASM
(window as any).direct_lon = direct_lon;
(window as any).demo_course_set = demo_course_set;

/* eslint-enable */
