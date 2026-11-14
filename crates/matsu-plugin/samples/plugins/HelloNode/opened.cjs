// The jack DocumentOpen (CommonJS): the process stays, so what is kept in `globalThis` is there
// at the next call.
globalThis.helloNodeOpens = (globalThis.helloNodeOpens ?? 0) + 1;
