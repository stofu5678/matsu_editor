# Generates the embed package of MicroPython for UME Editor (see vendor/micropython/README.md):
#
#   make -C crates/ume-micropython-sys/embed MICROPYTHON_TOP=<the micropython checkout> \
#        PACKAGE_DIR=<an empty directory for the result>
#
# The standard embed makefile (ports/embed/embed.mk) writes the core (py/), a few shared files
# and the generated headers. UME Editor also wants some extmod modules (json, binascii, heapq),
# so they join the scan for the generated headers and are copied by the extra step below.
# The C files of the crate (csrc/) are scanned too, because they use MP_QSTR_ names.

MICROPYTHON_TOP ?= ../../../../micropython
PACKAGE_DIR ?= micropython_embed

EXTMOD = \
	extmod/modjson.c \
	extmod/modbinascii.c \
	extmod/modheapq.c

SRC_QSTR += $(addprefix $(MICROPYTHON_TOP)/,$(EXTMOD))
SRC_QSTR += $(wildcard $(CURDIR)/../csrc/*.c)

include $(MICROPYTHON_TOP)/ports/embed/embed.mk

# `re` is not included: its engine (lib/re1.5) says "BSD-style license that can be found in the
# LICENSE file", but MicroPython ships no such file next to it, so its license text is missing.
.PHONY: ume-extra
all: ume-extra
ume-extra: micropython-embed-package
	$(ECHO) "- extmod modules and the license"
	$(Q)$(CP) $(addprefix $(TOP)/,$(EXTMOD)) $(PACKAGE_DIR)/extmod
	$(Q)$(CP) $(TOP)/LICENSE $(PACKAGE_DIR)/LICENSE
