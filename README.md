Simple mandelbrot generator writen in rust.

Uses two passes to generate the output image:
First - Perform the mandelbrot test
Second - Convert the results from first pass into color data

Currently the first and second pass are a very naive implementation.

Credits:
Color generation - adapted from https://github.com/huonw/simd