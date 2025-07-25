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
    <p>Header</p>
  </header>
  <main>
    <input id="picker" type="file" />
    <pre id="report"></pre>
    <div id="download"></div>
  </main>
  <aside>
    <p>Sidebar</p>
  </aside>
  <footer>
    <p>Footer</p>
  </footer>
`;

function setupPicker(p: HTMLInputElement) {
  p.addEventListener("change", async (e) => {
    const target = e.target as HTMLInputElement;
    const file: File | undefined = target.files?.[0];
    if (!file) return;

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
  <button id="downloadbtn" type="button">Download</button>
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
