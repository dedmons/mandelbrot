Simple mandelbrot generator writen in rust.

Uses two passes to generate the output image:
 1. Perform the mandelbrot test
 2. Convert the results from first pass into color data

Currently the first and second pass use a very naive approach.

**Credits**

Color generation - adapted from https://github.com/huonw/simd
