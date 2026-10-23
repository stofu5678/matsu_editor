// MicroPython includes <unistd.h> for `ssize_t`; Microsoft's compiler has no such header.
#ifndef MATSU_MSVC_UNISTD_H
#define MATSU_MSVC_UNISTD_H

#include <stddef.h>
#include <stdint.h>

typedef intptr_t ssize_t;

#endif
