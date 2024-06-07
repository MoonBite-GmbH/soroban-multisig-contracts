SUBDIRS := contracts/token contracts/multisig contracts/deployer
BUILD_FLAGS ?=

default: build

all: test

build:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir build BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
	done

test: build
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir test BUILD_FLAGS=$(BUILD_FLAGS) || exit 1; \
	done

fmt:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir fmt || exit 1; \
	done

lints: fmt
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir clippy || exit 1; \
	done

clean:
	@for dir in $(SUBDIRS) ; do \
		$(MAKE) -C $$dir clean || exit 1; \
	done
