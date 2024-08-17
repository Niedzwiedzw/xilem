// Copyright 2024 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use tracing::info_span;
use vello::kurbo::Vec2;

use crate::render_root::{RenderRoot, RenderRootState};
use crate::tree_arena::{ArenaMut, ArenaMutChildren};
use crate::{ComposeCtx, Widget, WidgetId, WidgetState};

fn recurse_on_children(
    id: WidgetId,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMutChildren<'_, WidgetState>,
    mut callback: impl FnMut(ArenaMut<'_, Box<dyn Widget>>, ArenaMut<'_, WidgetState>),
) {
    let parent_name = widget.item.short_type_name();
    let parent_id = id;

    for child_id in widget.item.children_ids() {
        let widget = widget
            .children
            .get_child_mut(child_id.to_raw())
            .unwrap_or_else(|| {
                panic!(
                    "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                    parent_name,
                    parent_id.to_raw(),
                    child_id.to_raw()
                )
            });
        let state = state.get_child_mut(child_id.to_raw()).unwrap_or_else(|| {
            panic!(
                "Error in '{}' #{}: cannot find child #{} returned by children_ids()",
                parent_name,
                parent_id.to_raw(),
                child_id.to_raw()
            )
        });

        callback(widget, state);
    }
}

fn compose_widget(
    global_state: &mut RenderRootState,
    mut widget: ArenaMut<'_, Box<dyn Widget>>,
    mut state: ArenaMut<'_, WidgetState>,
    parent_moved: bool,
    parent_translation: Vec2,
) {
    let _span = widget.item.make_trace_span().entered();

    let moved = parent_moved || state.item.translation_changed;
    let translation = parent_translation + state.item.translation + state.item.origin.to_vec2();
    state.item.window_origin = translation.to_point();

    let mut ctx = ComposeCtx {
        global_state,
        widget_state: state.item,
        widget_state_children: state.children.reborrow_mut(),
        widget_children: widget.children.reborrow_mut(),
    };
    if ctx.widget_state.request_compose {
        widget.item.compose(&mut ctx);
    }

    state.item.needs_compose = false;
    state.item.request_compose = false;
    state.item.translation_changed = false;

    let id = state.item.id;
    let parent_state = state.item;
    recurse_on_children(
        id,
        widget.reborrow_mut(),
        state.children,
        |widget, mut state| {
            if !moved && !state.item.translation_changed && !state.item.needs_compose {
                return;
            }
            compose_widget(
                global_state,
                widget,
                state.reborrow_mut(),
                moved,
                translation,
            );
            parent_state.merge_up(state.item);
        },
    );
}

// ----------------

pub fn root_compose(root: &mut RenderRoot, global_root_state: &mut WidgetState) {
    let _span = info_span!("compose").entered();

    let (root_widget, root_state) = root.widget_arena.get_pair_mut(root.root.id());
    compose_widget(&mut root.state, root_widget, root_state, false, Vec2::ZERO);

    global_root_state.merge_up(root.widget_arena.get_state_mut(root.root.id()).item);
}
