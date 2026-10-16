// MicroPython includes <unistd.h> for `ssize_t`; Microsoft's compiler has no such header.
#ifndef UME_MSVC_UNISTD_H
#define UME_MSVC_UNISTD_H

#include <stddef.h>
#include <stdint.h>

typedef intptr_t ssize_t;

#endif
