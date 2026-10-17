# Generates the embed package of MicroPython for UME Editor (see vendor/micropython/README.md):
#
#   make -C crates/ume-micropython-sys/embed MICROPYTHON_TOP=<the micropython checkout> \
#        PACKAGE_DIR=<an empty directory for the result>
#
# The standard embed makefile (ports/embed/embed.mk) writes the core (py/), a few shared files
# and the generated headers. UME Editor also wants some extmod modules (json, binascii, heapq, re),
# so they join the scan for the generated headers and are copied by the extra step below.
# The C files of the crate (csrc/) are scanned too, because they use MP_QSTR_ names.

MICROPYTHON_TOP ?= ../../../../micropython
PACKAGE_DIR ?= micropython_embed

EXTMOD = \
	extmod/modjson.c \
	extmod/modbinascii.c \
	extmod/modheapq.c \
	extmod/modre.c

SRC_QSTR += $(addprefix $(MICROPYTHON_TOP)/,$(EXTMOD))
SRC_QSTR += $(wildcard $(CURDIR)/../csrc/*.c)

include $(MICROPYTHON_TOP)/ports/embed/embed.mk

# modre.c #include's the engine's C files (lib/re1.5/*.c), so the engine is copied with it.
# Its license text is not in MicroPython; it comes from the upstream re1.5 project and is added
# by hand (see vendor/micropython/README.md).
RE15 = lib/re1.5/charclass.c lib/re1.5/compilecode.c lib/re1.5/dumpcode.c \
	lib/re1.5/re1.5.h lib/re1.5/recursiveloop.c

.PHONY: ume-extra
all: ume-extra
ume-extra: micropython-embed-package
	$(ECHO) "- extmod modules and the license"
	$(Q)$(CP) $(addprefix $(TOP)/,$(EXTMOD)) $(PACKAGE_DIR)/extmod
	$(Q)$(MKDIR) -p $(PACKAGE_DIR)/lib/re1.5
	$(Q)$(CP) $(addprefix $(TOP)/,$(RE15)) $(PACKAGE_DIR)/lib/re1.5
	$(Q)$(CP) $(TOP)/LICENSE $(PACKAGE_DIR)/LICENSE
