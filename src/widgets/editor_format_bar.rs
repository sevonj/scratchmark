mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use gtk::gio;
    use gtk::glib;
    use gtk::prelude::*;

    use gio::SimpleAction;
    use gio::SimpleActionGroup;
    use glib::clone;
    use gtk::CompositeTemplate;

    use crate::widgets::Editor;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/editor_format_bar.ui")]
    pub struct EditorFormatBar {
        actions: SimpleActionGroup,
        pub(super) editor: RefCell<Option<Editor>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EditorFormatBar {
        const NAME: &'static str = "EditorFormatBar";
        type Type = super::EditorFormatBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EditorFormatBar {
        fn constructed(&self) {
            self.parent_constructed();

            self.setup_actions();
        }
    }

    impl WidgetImpl for EditorFormatBar {}
    impl BinImpl for EditorFormatBar {}

    impl EditorFormatBar {
        fn setup_actions(&self) {
            let obj = self.obj();

            obj.insert_action_group("formatbar", Some(&self.actions));

            let action = SimpleAction::new("bold", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor.activate_action("editor.format-bold", None).unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("italic", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-italic", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("strikethrough", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-strikethrough", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("heading", Some(glib::VariantTy::INT32));
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, size| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-heading", size)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("blockquote", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-blockquote", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("code", None);
            action.connect_activate(clone!(
                #[weak(rename_to = this)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = this.editor.borrow().as_ref() {
                        editor.activate_action("editor.format-code", None).unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            self.update_enabled();
        }

        pub(super) fn update_enabled(&self) {
            let enabled = self.editor.borrow().is_some();

            for action in self.actions.list_actions() {
                let act: SimpleAction = self
                    .actions
                    .lookup_action(&action)
                    .unwrap()
                    .downcast()
                    .unwrap();
                act.set_enabled(enabled);
            }
        }
    }
}

use adw::subclass::prelude::ObjectSubclassIsExt;
use glib::Object;
use gtk::glib;

use crate::widgets::Editor;

glib::wrapper! {
    pub struct EditorFormatBar(ObjectSubclass<imp::EditorFormatBar>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for EditorFormatBar {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl EditorFormatBar {
    pub fn bind_editor(&self, editor: Option<Editor>) {
        self.imp().editor.replace(editor);
        self.imp().update_enabled();
    }
}
