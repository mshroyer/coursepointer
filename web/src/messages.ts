import type { OptionValues } from "./options.ts";
import type { JsConversionInfo } from "coursepointer-wasm";

/**
 * The category of a request or response to/from the worker.
 */
export enum WorkerMessage {
  /**
   * Indicates that the worker has initialized and is ready to handle messages.
   */
  Ready,

  /**
   * Request/result of GPX to FIT conversion.
   */
  ConvertGpxToFit,
}

export class WorkerRequest {
  readonly type: WorkerMessage;

  constructor(type: WorkerMessage) {
    this.type = type;
  }

  static setPrototype(req: WorkerRequest) {
    let prototype;
    switch (req.type) {
      case WorkerMessage.ConvertGpxToFit:
        prototype = ConvertGpxToFitRequest.prototype;
        break;
    }
    if (prototype !== undefined) {
      Object.setPrototypeOf(req, prototype);
    }
  }
}

export class WorkerResponse {
  readonly type: WorkerMessage;
  err?: Error;

  constructor(type: WorkerMessage, err?: Error) {
    this.type = type;
    this.err = err;
  }

  static setPrototype(req: WorkerResponse) {
    let prototype;
    switch (req.type) {
      case WorkerMessage.Ready:
        prototype = ReadyResponse.prototype;
        break;
      case WorkerMessage.ConvertGpxToFit:
        prototype = ConvertGpxToFitResponse.prototype;
        break;
    }
    if (prototype !== undefined) {
      Object.setPrototypeOf(req, prototype);
    }
  }
}

export class ConvertGpxToFitRequest extends WorkerRequest {
  course: Uint8Array;
  options: OptionValues;

  constructor(course: Uint8Array, options: OptionValues) {
    super(WorkerMessage.ConvertGpxToFit);
    this.course = course;
    this.options = options;
  }
}

export class ConvertGpxToFitResponse extends WorkerResponse {
  info?: JsConversionInfo;

  constructor(info?: JsConversionInfo, err?: Error) {
    super(WorkerMessage.ConvertGpxToFit, err);
    this.info = info;
  }
}

export class ReadyResponse extends WorkerResponse {
  sports: any[];

  constructor(sports: any[]) {
    super(WorkerMessage.Ready, undefined);
    this.sports = sports;
  }
}
