# Reading and navigating an investigation

Graph, Data, Code, the tree and Harness Insight refer to the same canonical record IDs. Layout changes only presentation coordinates. A numbered graph card identifies its record kind in text; color is supplemental. Inferred explanations, hypotheses and unknowns retain their original distinctions in the inspector.

## Start with a readable record

**Center selected** restores a readable scale around the selected record. It does not fit distant citations into the same tiny view. **Fit scope** gives an overview. At small scales, cards show readable IDs instead of illegible full titles; center a card or use the tree to read its full explanation.

- **All** shows the available investigation records.
- **Branch** follows recorded question ownership, AI request ancestry and review targets for the selected branch.
- **Neighbors** shows the selected record and its direct recorded relationships. Choosing this scope brings unpinned neighbors together.

The scope anchor stays fixed until explicitly changed. Selecting another record does not silently change the branch; centering a record outside the current scope opens its neighbors. Counts distinguish records and links inside the scope from links crossing its boundary. A replay only exposes available records; missing links do not imply that no relationship exists.

**Selected relationships / accessible list** supplies keyboard-operable destination controls, typed relationship names, their purpose and available coverage. Byte coverage is separate from semantic certainty. Unknowns remain unresolved; the graph does not invent missing destinations or recovered functions.

## Layout and pins

Narrative groups explicit question branches in recorded order. Relationships runs a deterministic force calculation once and separates overlapping unpinned cards. It does not animate continuously. Scoped arrangements keep small neighbor sets together. Dragging or moving a card pins it; **Pin selected** toggles its pin. Pins survive layout and scope changes in the current window. Two deliberately overlapping pinned cards remain where the analyst put them.

Positions and pins are window presentation state, not saved evidence. Reopening rebuilds a layout without changing any IDs, notes or relationships. Live results append without moving existing cards or camera bounds. Reading state and presenter hold are described in [harness memory](harness-memory.md).

## Keyboard and accessibility

| Action | Keyboard |
| --- | --- |
| Find an ID, title, kind or summary | Ctrl+F, type, Enter |
| Leave the search field | Escape |
| Center selected / fit scope | Alt+C / Alt+F |
| Zoom in / out | Alt+= / Alt+- |
| Pan | Alt+arrow keys |
| Previous / next record | Alt+PageUp / Alt+PageDown |
| Pin selected | Alt+P |
| Move selected card and pin it | Alt+Shift+arrow keys |
| Traverse controls and the tree | Tab / Shift+Tab, then Enter or Space |

The **Navigate** menu supplies named alternatives to graph gestures. Scope and layout selectors are normal egui controls. Canvas cards expose IDs, kinds, titles and summaries to AccessKit; the tree provides an accessible index independent of zoom. **View > Panel widths** offers sliders and numeric fields for both side panes, in addition to the resize borders. Wheel scrolling pans the canvas; Ctrl+wheel zooms through egui's Scene primitive.

## Bounds and checks

The validated case limit is 200 records and 1,000 edges. Force relaxation has 160 bounded iterations, followed by bounded overlap separation. Neither runs on live arrivals. Cards outside the viewport are culled; relationship endpoints use stable ID lookups. Zoom is clamped to 8–160%. Full labels occupy at most two title rows and one summary row, with full text in the inspector and accessible metadata.

Tests cover canonical-record immutability, deterministic settled positions, pins, overlap separation, live camera preservation, replay scopes, zoom limits and actual egui pointer events at 1280x720 and 1920x1080. Native observations and automation limitations are recorded in [native validation](native-validation.md).
