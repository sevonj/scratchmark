mod imp {
    use std::cell::RefCell;

    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;
    use gtk::gio::SimpleAction;
    use gtk::gio::SimpleActionGroup;
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::prelude::*;

    use crate::widgets::Editor;

    #[derive(CompositeTemplate, Default)]
    #[template(resource = "/org/scratchmark/Scratchmark/ui/markdown_format_bar.ui")]
    pub struct MarkdownFormatBar {
        actions: SimpleActionGroup,
        pub(super) editor: RefCell<Option<Editor>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MarkdownFormatBar {
        const NAME: &'static str = "MarkdownFormatBar";
        type Type = super::MarkdownFormatBar;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for MarkdownFormatBar {
        fn constructed(&self) {
            self.parent_constructed();

            self.setup_actions();
        }
    }

    impl WidgetImpl for MarkdownFormatBar {}
    impl BinImpl for MarkdownFormatBar {}

    impl MarkdownFormatBar {
        fn setup_actions(&self) {
            let obj = self.obj();

            obj.insert_action_group("formatbar", Some(&self.actions));

            let action = SimpleAction::new("bold", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor.activate_action("editor.format-bold", None).unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("italic", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-italic", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("strikethrough", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-strikethrough", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("highlight", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-highlight", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("heading", Some(glib::VariantTy::INT32));
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, size| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-heading", size)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("blockquote", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
                        editor
                            .activate_action("editor.format-blockquote", None)
                            .unwrap();
                    }
                }
            ));
            self.actions.add_action(&action);

            let action = SimpleAction::new("code", None);
            action.connect_activate(clone!(
                #[weak(rename_to = imp)]
                self,
                move |_action, _parameter| {
                    if let Some(editor) = imp.editor.borrow().as_ref() {
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

use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::Object;

use crate::widgets::Editor;

glib::wrapper! {
    pub struct MarkdownFormatBar(ObjectSubclass<imp::MarkdownFormatBar>)
        @extends adw::Bin, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for MarkdownFormatBar {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl MarkdownFormatBar {
    pub fn bind_editor(&self, editor: Option<Editor>) {
        self.imp().editor.replace(editor);
        self.imp().update_enabled();
    }
}
