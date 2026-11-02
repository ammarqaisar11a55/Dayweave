# Design system

Dayweave's visual identity comes from its name. In weaving, the **warp** threads are held fixed on
the loom and the **weft** passes through them. A day has the same structure: habits are the fixed
threads and tasks are what passes through them.

## The weave

The Overview's progress visual is a small piece of plain weave. Vertical threads are today's habits,
horizontal threads are today's tasks, and each thread fills with orange when that item is done. As
the day goes on the cloth fills in. A ring chart would report the same number, but this one shows
what kind of work is left, and it could not belong to any other product.

The logo is the same structure at its smallest: four threads locked in a pinwheel.

This is the one expressive element in the application. Everything around it stays quiet.

## Colour

The palette is specified by the brief. These are the working tokens, defined in `src/styles/tokens.css`.

| Token | Dark | Light |
| --- | --- | --- |
| Window | `#171717` | `#F7F7F8` |
| Sidebar | `#1D1D1D` | `#FFFFFF` |
| Card | `#252525` | `#FFFFFF` |
| Elevated | `#2B2B2B` | `#FFFFFF` |
| Border | `#383838` | `#E8E8EA` |
| Primary text | `#F5F5F5` | `#202124` |
| Secondary text | `#A5A5A5` | `#6C6C72` |
| Accent | `#E95420` | `#E95420` |
| Accent hover | `#F16A35` | `#F16A35` |
| Accent as text | `#F4764A` | `#C2410F` |
| Ink on accent | `#1F0A02` | `#1F0A02` |

### Changes made for contrast

The brief asks for specific colours and also for sufficient contrast. Where the two conflicted, the
following changes keep the brand colour and meet WCAG AA for text.

| Pair | Ratio | Decision |
| --- | --- | --- |
| White on `#E95420` | 3.65 | Fails, and the lighter hover colour lowers it to 3.06. Filled buttons use dark ink instead: 5.22, rising to 6.22 on hover |
| `#E95420` as small text | 3.65 on white, 4.20 on a dark card | Text uses a darker or lighter orange per theme. `#E95420` stays for fills, marks and indicators |
| `#77777D` secondary text | 4.16 on the light window | Changed to `#6C6C72`: 4.87 |

Semantic colours are muted and always appear beside an icon or a word, so status never depends on
colour alone.

| Meaning | Dark | Light |
| --- | --- | --- |
| Success, habits | `#4CBB87` | `#1B7F54` |
| Warning | `#E5B23C` | `#8F6400` |
| Error, overdue | `#F0776F` | `#BF2F2B` |
| Information, tasks | `#6FAEEB` | `#2267AE` |
| Focus sessions | `#B19AE8` | `#6A49B8` |

## Type

Inter, bundled with the application so nothing is fetched, falling back to `system-ui` and Ubuntu.
Numbers use tabular figures wherever they change or line up.

| Role | Size and weight |
| --- | --- |
| Page heading | 28px, 650 |
| Section heading | 17px, 600 |
| Card heading | 15px, 600 |
| Body | 14px, 400 |
| Secondary label | 13px, 400 |
| Metadata | 12px, 400 |

Headings use sentence case. There are no all-capitals labels.

## Space and shape

Spacing uses 4, 8, 12, 16, 24, 32 and 48px.

Radius follows hierarchy rather than being one value everywhere:

| Radius | Used for |
| --- | --- |
| 8px | Chips, badges, small controls, list rows |
| 12px | Inputs, buttons, menus |
| 18px | Grouped lists and sections |
| 24px | Dialogs |

Surfaces separate by tone, not shadow. Cards have no shadow in either theme, and only floating layers
(menus, dialogs, the palette) are raised. Blur is used only behind those layers.

## Layout

Grouped lists follow One UI's and GNOME's boxed-list idea: related rows share one rounded surface,
and rows inside it have no borders of their own. A dashboard made of many identical bordered cards is
the look to avoid. Pages open with a large heading and generous space, then get denser.

## Motion

All transitions take 120 to 220ms and share one easing curve, `cubic-bezier(.2, .8, .2, 1)`. Motion
answers something you did: a thread filling, a panel opening, a row settling. Nothing moves on its own
except the focus timer. `prefers-reduced-motion` removes the movement, and a setting forces this on.

## Writing

Plain verbs and sentence case. A control says exactly what it does, and an action keeps its name
through the flow: "Archive" produces "Task archived". Empty screens say what to do next, and errors say
what happened and how to fix it.
