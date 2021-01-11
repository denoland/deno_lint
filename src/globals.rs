// Copyright 2020 the Deno authors. All rights reserved. MIT license.

/// List of globals available in Deno environment.
///
/// Boolean tells if global can be overwritten
///
/// Adapted from https://www.npmjs.com/package/globals
pub static GLOBALS: &[(&str, bool)] = &[
  ("AbortController", false),
  ("AbortSignal", false),
  ("addEventListener", false),
  ("AggregateError", false),
  ("alert", false),
  ("Array", false),
  ("ArrayBuffer", false),
  ("atob", false),
  ("Atomics", false),
  ("BigInt", false),
  ("BigInt64Array", false),
  ("BigUint64Array", false),
  ("Blob", false),
  ("Boolean", false),
  ("btoa", false),
  ("ByteLengthQueuingStrategy", false),
  ("clearInterval", false),
  ("clearTimeout", false),
  ("close", false),
  ("closed", false),
  ("CloseEvent", false),
  ("confirm", false),
  ("console", false),
  ("constructor", false),
  ("CountQueuingStrategy", false),
  ("crypto", false),
  ("CustomEvent", false),
  ("DataView", false),
  ("Date", false),
  ("decodeURI", false),
  ("decodeURIComponent", false),
  ("DedicatedWorkerGlobalScope", false),
  ("Deno", false),
  ("dispatchEvent", false),
  ("DOMException", false),
  ("encodeURI", false),
  ("encodeURIComponent", false),
  ("Error", false),
  ("ErrorEvent", false),
  ("escape", false),
  ("eval", false),
  ("EvalError", false),
  ("Event", false),
  ("EventTarget", false),
  ("fetch", false),
  ("File", false),
  ("FileReader", false),
  ("FinalizationRegistry", false),
  ("Float32Array", false),
  ("Float64Array", false),
  ("FormData", false),
  ("Function", false),
  ("globalThis", false),
  ("hasOwnProperty", false),
  ("Headers", false),
  ("Infinity", false),
  ("Int16Array", false),
  ("Int32Array", false),
  ("Int8Array", false),
  ("isFinite", false),
  ("isNaN", false),
  ("isPrototypeOf", false),
  ("JSON", false),
  ("location", false),
  ("Location", false),
  ("Map", false),
  ("Math", false),
  ("MessageEvent", false),
  ("NaN", false),
  ("Number", false),
  ("Object", false),
  ("onerror", true),
  ("onload", true),
  ("onmessage", true),
  ("onmessageerror", true),
  ("onunload", true),
  ("parseFloat", false),
  ("parseInt", false),
  ("performance", false),
  ("Performance", false),
  ("PerformanceEntry", false),
  ("PerformanceMark", false),
  ("PerformanceMeasure", false),
  ("Permissions", false),
  ("PermissionStatus", false),
  ("postMessage", true),
  ("ProgressEvent", false),
  ("Promise", false),
  ("prompt", false),
  ("propertyIsEnumerable", false),
  ("Proxy", false),
  ("queueMicrotask", false),
  ("RangeError", false),
  ("ReadableStream", false),
  ("ReferenceError", false),
  ("Reflect", false),
  ("RegExp", false),
  ("removeEventListener", false),
  ("Request", false),
  ("Response", false),
  ("self", false),
  ("Set", false),
  ("setInterval", false),
  ("setTimeout", false),
  ("SharedArrayBuffer", false),
  ("String", false),
  ("Symbol", false),
  ("SyntaxError", false),
  ("TextDecoder", false),
  ("TextEncoder", false),
  ("toLocaleString", false),
  ("toString", false),
  ("TransformStream", false),
  ("TypeError", false),
  ("Uint16Array", false),
  ("Uint32Array", false),
  ("Uint8Array", false),
  ("Uint8ClampedArray", false),
  ("undefined", false),
  ("unescape", false),
  ("URIError", false),
  ("URL", false),
  ("URLSearchParams", false),
  ("valueOf", false),
  ("WeakMap", false),
  ("WeakRef", false),
  ("WeakSet", false),
  ("WebAssembly", false),
  ("WebSocket", false),
  ("window", false),
  ("Window", false),
  ("Worker", false),
  ("WorkerGlobalScope", false),
  ("WorkerLocation", false),
  ("WritableStream", false),
];
