// @ts-ignore
import geographicLib from './wasm/geographiclib.mjs'
import init, {demo_course_set, direct_lon, read_gpx_bytes} from "coursepointer-wasm";

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <input id="picker" type="file" />
    <div id="output"></div>
  </div>
`;

function setupPicker(p: HTMLInputElement) {
    p.addEventListener('change', async e => {
        const target = e.target as HTMLInputElement;
        const file : File | undefined = target.files?.[0];
        if (!file) return;

        const buf = await file.arrayBuffer();
        console.time('read_gpx_bytes');
        let course = read_gpx_bytes(new Uint8Array(buf));
        console.timeEnd('read_gpx_bytes');
        let len_m = 0;
        if (course.records.length > 0) {
            len_m = course.records[course.records.length-1].cumulative_distance_m;
        }
        document.querySelector<HTMLDivElement>('#output')!.innerHTML = len_m.toString();
    })
}

setupPicker(document.querySelector<HTMLInputElement>('#picker')!);

const GEO = await geographicLib();
(window as any).GEO = GEO;
await init();

// Functions exported by coursepointer WASM
(window as any).direct_lon = direct_lon;
(window as any).demo_course_set = demo_course_set;

