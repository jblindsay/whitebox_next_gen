// We need to forward routine registration from the package shared library
// to the Rust extendr module linked from the wbw_r static library.

void R_init_wbw_r_extendr(void *dll);

void R_init_whiteboxworkflows(void *dll) {
    R_init_wbw_r_extendr(dll);
}
