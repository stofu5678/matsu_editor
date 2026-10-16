# Build configuration used only to GENERATE the files that crates/ume-mruby-sys vendors
# (see ../README.md). The library itself is compiled by build.rs with the `cc` crate, so any
# change to the defines or the gems here must be mirrored in build.rs.
MRuby::Build.new do |conf|
  conf.toolchain
  conf.build_dir = ENV.fetch('UME_MRUBY_BUILD_DIR')

  # Strings are UTF-8 (String#length counts characters, not bytes).
  conf.defines << 'MRB_UTF8_STRING'

  conf.gem core: 'mruby-compiler'
  conf.gem core: 'mruby-error'
  conf.gem core: 'mruby-bigint'
  conf.gembox 'stdlib'
  conf.gembox 'stdlib-ext'
  conf.gem core: 'mruby-math'
  conf.gem core: 'mruby-strftime'
  conf.gem core: 'mruby-metaprog'
  conf.gem core: 'mruby-method'
  conf.gem core: 'mruby-eval'
end
