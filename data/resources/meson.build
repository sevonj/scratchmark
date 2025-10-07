pkgdatadir = get_option('prefix') / get_option('datadir') / meson.project_name()

gnome.compile_resources('scratchmark',
  'gresources.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
)
