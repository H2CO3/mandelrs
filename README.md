# Mandelbrot set solver

This is a Mandelbrot set solver that I created in a few hours
just for fun.

The following optimizations are applied:

* Vertical symmetry of the Mandelbrot set is observed. If a complex
  number is in the Mandelbrot set, then so is its conjugate, so it
  does not have to be computed separately.
* The maximal circle fitting into the big, central cardioid, and
  the smaller circle centered at -1 are checked as a fast path.
  This turns out to result in another ~50% speedup.
* The computations are performed in 4.28 signed fixpoint format,
  which was approximately 20...30% faster than using `f32`.
* 8 threads are carrying out the computation in parallel, since
  each point can be tested for membership independent from every
  other one. The region [-1.5…+0.5] x [-1.0…+1.0] is divided
  into 16 x 16 pixel blocks. Each block's upper left coordinate is
  pushed onto a single-producer, multiple-consumer channel.
  The threads read it out as a FIFO, on a "first come, first served"
  basis, which means that there is negligible time when a thread
  is waiting on join for the others to finish, rather than doing
  actual work.

On my 2014 MacBook Pro (2.2 GHz), it takes approximately 10...12
milliseconds to solve the set, and another 10 or so milliseconds
to actually render the result onto the SDL window.

## Building and Running

This project depends on SDL2. Therefore, you must tell Cargo how
to link against the SDL2 libraries. The Rust SDL2 bindings crate
documents the process very well. For example, on macOS, if you
installed SDL2 via Homebrew, it is quite simple:

    LIBRARY_PATH="$LIBRARY_PATH:/usr/local/lib" cargo run --release
