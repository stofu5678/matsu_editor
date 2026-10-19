//! The fields of the save panel of macOS ([`Request::extras`](super::Request::extras)): the
//! `accessoryView` of `NSSavePanel`, a vertical stack of labels, pop-up lists and check boxes. Not
//! run by CI (CI only builds it); the user checks it on a Mac.

#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol};
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{
    NSButton, NSControlStateValueOff, NSControlStateValueOn, NSLayoutAttribute, NSPopUpButton,
    NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
};
use objc2_foundation::{NSArray, NSPoint, NSRect, NSSize, NSString};

use super::{Extra, ExtraValue, Follow};

/// One field that was made, to read back.
enum Control {
    Combo(Retained<NSPopUpButton>),
    Check(Retained<NSButton>),
}

/// The fields in the panel. Keep it alive until the panel is closed: the panel does not own the
/// object that answers a choice in a list.
pub(super) struct Fields {
    pub view: Retained<NSView>,
    controls: Vec<Control>,
    _target: Option<Retained<Follower>>,
}

impl Fields {
    pub(super) fn new(
        mtm: MainThreadMarker,
        extras: &[Extra],
        follow: Option<&Follow>,
    ) -> Option<Self> {
        if extras.is_empty() {
            return None;
        }
        let mut views: Vec<Retained<NSView>> = Vec::new();
        let mut controls = Vec::new();
        for extra in extras {
            match extra {
                Extra::Combo {
                    label,
                    items,
                    selected,
                } => {
                    let text = NSTextField::labelWithString(&NSString::from_str(label), mtm);
                    views.push(Retained::into_super(Retained::into_super(text)));
                    let list = NSPopUpButton::initWithFrame_pullsDown(
                        mtm.alloc(),
                        NSRect::new(NSPoint::ZERO, NSSize::new(220., 26.)),
                        false,
                    );
                    for item in items {
                        list.addItemWithTitle(&NSString::from_str(item));
                    }
                    list.selectItemAtIndex(*selected as isize);
                    views.push(Retained::into_super(Retained::into_super(
                        Retained::into_super(list.clone()),
                    )));
                    controls.push(Control::Combo(list));
                }
                Extra::Check {
                    label,
                    checked,
                    enabled,
                } => {
                    // SAFETY: no target and no action: a plain check box.
                    let check = unsafe {
                        NSButton::checkboxWithTitle_target_action(
                            &NSString::from_str(label),
                            None,
                            None,
                            mtm,
                        )
                    };
                    check.setState(state_of(*checked));
                    check.setEnabled(*enabled);
                    views.push(Retained::into_super(Retained::into_super(check.clone())));
                    controls.push(Control::Check(check));
                }
            }
        }
        let stack = NSStackView::stackViewWithViews(&NSArray::from_retained_slice(&views), mtm);
        stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        stack.setAlignment(NSLayoutAttribute::Leading);
        stack.setSpacing(6.);
        let view: Retained<NSView> = Retained::into_super(stack);
        view.setFrameSize(view.fittingSize());
        // BOM in step with the encoding (`OnItemSelected` of Sakura): the list tells the target
        let target = follow.and_then(|follow| {
            let (Some(Control::Combo(list)), Some(Control::Check(check))) =
                (controls.get(follow.combo), controls.get(follow.check))
            else {
                return None;
            };
            let target = Follower::new(mtm, follow.clone(), list.clone(), check.clone());
            let any: &AnyObject = &target;
            // SAFETY: `choose:` is defined by `Follower` with this one argument.
            unsafe {
                list.setTarget(Some(any));
                list.setAction(Some(sel!(choose:)));
            }
            Some(target)
        });
        Some(Self {
            view,
            controls,
            _target: target,
        })
    }

    /// The value of every field, in the order of the request.
    pub(super) fn values(&self) -> Vec<ExtraValue> {
        self.controls
            .iter()
            .map(|control| match control {
                Control::Combo(list) => {
                    ExtraValue::Combo(list.indexOfSelectedItem().max(0) as usize)
                }
                Control::Check(check) => ExtraValue::Check(check.state() == NSControlStateValueOn),
            })
            .collect()
    }
}

fn state_of(on: bool) -> isize {
    if on {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    }
}

struct FollowerState {
    follow: Follow,
    list: Retained<NSPopUpButton>,
    check: Retained<NSButton>,
}

define_class!(
    // SAFETY: NSObject has no requirements for subclasses that this breaks, and Follower does not
    // implement Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "UmeSaveFieldsFollower"]
    #[ivars = FollowerState]
    struct Follower;

    unsafe impl NSObjectProtocol for Follower {}

    impl Follower {
        /// The action of the list: the check box takes the state the rule says for the item.
        #[unsafe(method(choose:))]
        fn choose(&self, _sender: Option<&AnyObject>) {
            let state = self.ivars();
            let index = state.list.indexOfSelectedItem().max(0) as usize;
            let (enabled, checked) = (state.follow.rule)(index);
            state.check.setEnabled(enabled);
            state.check.setState(state_of(checked));
        }
    }
);

impl Follower {
    fn new(
        mtm: MainThreadMarker,
        follow: Follow,
        list: Retained<NSPopUpButton>,
        check: Retained<NSButton>,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(FollowerState {
            follow,
            list,
            check,
        });
        // SAFETY: NSObject's initializer.
        unsafe { msg_send![super(this), init] }
    }
}
