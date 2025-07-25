#include "calzone.h"

#include <cstdio>
#if defined(_WIN32)
#define setenv(name, value, overwrite) _putenv_s(name, value)
#endif

void set_env(rust::String name, rust::String value) {
    setenv(name.c_str(), value.c_str(), 0);
}
