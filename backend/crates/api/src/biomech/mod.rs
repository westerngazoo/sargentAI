//! R-0059 / SPEC-0059 — lift biomechanics over HTTP. Stateless: the request
//! carries the lifter's measurements, the response carries the model's
//! frames, and nothing is stored or logged (AC2, AC8).

mod lever;
