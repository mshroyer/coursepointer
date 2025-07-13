import './style.css'
import typescriptLogo from './typescript.svg'
import viteLogo from '/vite.svg'
import { setupCounter } from './counter.ts'
// @ts-ignore
import geographicLib from './wasm/geographiclib.mjs'
import init, {demo_course_set, direct_lon} from "coursepointer-wasm";

document.querySelector<HTMLDivElement>('#app')!.innerHTML = `
  <div>
    <a href="https://vite.dev" target="_blank">
      <img src="${viteLogo}" class="logo" alt="Vite logo" />
    </a>
    <a href="https://www.typescriptlang.org/" target="_blank">
      <img src="${typescriptLogo}" class="logo vanilla" alt="TypeScript logo" />
    </a>
    <h1>Vite + TypeScript</h1>
    <div class="card">
      <button id="counter" type="button"></button>
    </div>
    <p class="read-the-docs">
      Click on the Vite and TypeScript logos to learn more
    </p>
  </div>
`;

setupCounter(document.querySelector<HTMLButtonElement>('#counter')!);

const GEO = await geographicLib();

(window as any).geographicLib = geographicLib;
(window as any).GEO = GEO;

(window as any).SHIM = {};
function geodesic_direct(lat1: number, lon1: number, azi1: number, s12: number): any {
    return GEO.geodesicDirect(lat1, lon1, azi1, s12);
}
(window as any).SHIM.geodesic_direct = geodesic_direct;

await init();

(window as any).direct_lon = direct_lon;
(window as any).demo_course_set = demo_course_set;

