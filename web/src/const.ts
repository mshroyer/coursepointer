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
