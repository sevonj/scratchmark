mod imp {
    use adw::AlertDialog;
    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::glib;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor/file_changed_on_disk_dialog.ui")]
    pub struct FileChangedOnDiskDialog {}

    #[glib::object_subclass]
    impl ObjectSubclass for FileChangedOnDiskDialog {
        const NAME: &'static str = "FileChangedOnDiskDialog";
        type Type = super::FileChangedOnDiskDialog;
        type ParentType = AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FileChangedOnDiskDialog {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for FileChangedOnDiskDialog {}
    impl AdwDialogImpl for FileChangedOnDiskDialog {}
    impl AdwAlertDialogImpl for FileChangedOnDiskDialog {}
}

use adw::AlertDialog;
use adw::prelude::*;
use gtk::glib;
use gtk::glib::Object;

glib::wrapper! {
    pub struct FileChangedOnDiskDialog(ObjectSubclass<imp::FileChangedOnDiskDialog>)
        @extends adw::AlertDialog, adw::Dialog, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl Default for FileChangedOnDiskDialog {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl FileChangedOnDiskDialog {
    pub fn present(&self, parent: Option<&impl glib::object::IsA<gtk::Widget>>) {
        self.clone().upcast::<AlertDialog>().present(parent);
    }
}
