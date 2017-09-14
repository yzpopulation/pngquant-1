LIQSRCDIR ?= lib
BIN ?= pngquant
BINPREFIX ?= $(DESTDIR)$(PREFIX)/bin
MANPREFIX ?= $(DESTDIR)$(PREFIX)/share/man

DISTFILES = *.[chm] pngquant.1 Makefile configure README.md INSTALL CHANGELOG COPYRIGHT Cargo.toml
TARNAME = pngquant-$(VERSION)
TARFILE = $(TARNAME)-src.tar.gz

LIBDISTFILES = $(LIQSRCDIR)/*.[ch] $(LIQSRCDIR)/COPYRIGHT $(LIQSRCDIR)/README.md $(LIQSRCDIR)/configure $(LIQSRCDIR)/Makefile $(LIQSRCDIR)/Cargo.toml

all: $(BIN)

$(BIN)::
	MACOSX_DEPLOYMENT_TARGET=10.7 cargo build --release
	cp target/release/pngquant "$@"

test:
	cargo test

dist: $(TARFILE)

$(TARFILE): $(DISTFILES)
	make -C $(LIQSRCDIR) cargo
	rm -rf $(TARFILE) $(TARNAME)
	mkdir -p $(TARNAME)/lib/rust $(TARNAME)/lib/msvc-dist $(TARNAME)/rust
	cp $(DISTFILES) $(TARNAME)
	cp rust/*.rs $(TARNAME)/rust/
	cp $(LIBDISTFILES) $(TARNAME)/lib
	cp $(LIQSRCDIR)/rust/*.rs $(TARNAME)/lib/rust/
	cp $(LIQSRCDIR)/msvc-dist/*.[ch] $(TARNAME)/lib/msvc-dist/
	tar -czf $(TARFILE) --numeric-owner --exclude='._*' $(TARNAME)
	rm -rf $(TARNAME)
	-shasum $(TARFILE)

install: $(BIN) $(BIN).1
	-mkdir -p '$(BINPREFIX)'
	-mkdir -p '$(MANPREFIX)/man1'
	install -m 0755 -p '$(BIN)' '$(BINPREFIX)/$(BIN)'
	install -m 0644 -p '$(BIN).1' '$(MANPREFIX)/man1/'

uninstall:
	rm -f '$(BINPREFIX)/$(BIN)'
	rm -f '$(MANPREFIX)/man1/$(BIN).1'

clean:
	-test -n '$(LIQSRCDIR)' && $(MAKE) -C '$(LIQSRCDIR)' clean
	rm -f '$(BIN)' $(TARFILE)
	cargo clean

distclean: clean
	-test -n '$(LIQSRCDIR)' && $(MAKE) -C '$(LIQSRCDIR)' distclean
	rm -f config.mk pngquant-*-src.tar.gz

.PHONY: all clean dist distclean install uninstall test
.DELETE_ON_ERROR:
