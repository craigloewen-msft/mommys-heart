//! Lightweight, dependency-free chart components rendered with SVG + Tailwind.
//!
//! These are intentionally simple and fully reactive: each takes an owned
//! `Vec<MetricSlice>` (or scalar) and renders it, so callers can recompute the
//! data inside a reactive closure and the chart re-renders. No JS interop, no
//! canvas — keeps the WASM bundle small and the styling consistent with the app.

use leptos::prelude::*;

use crate::analytics::MetricSlice;

/// Fallback palette (Tailwind text-colour classes) used when a slice has no
/// explicit colour. Applied to SVG via `stroke="currentColor"` / `fill`.
const PALETTE: [&str; 8] = [
    "text-primary-400",
    "text-sky-400",
    "text-violet-400",
    "text-emerald-400",
    "text-amber-400",
    "text-rose-400",
    "text-cyan-400",
    "text-fuchsia-400",
];

/// Format a metric value: integers without decimals, otherwise one decimal.
fn fmt_num(v: f64) -> String {
    if (v.fract()).abs() < 0.05 {
        format!("{}", v.round() as i64)
    } else {
        format!("{v:.1}")
    }
}

fn color_for(slice: &MetricSlice, idx: usize) -> &'static str {
    slice.color.unwrap_or(PALETTE[idx % PALETTE.len()])
}

/// A headline number card with a label and optional sublabel.
#[component]
pub fn StatCard(
    #[prop(into)] label: String,
    #[prop(into)] value: String,
    #[prop(into, optional)] sublabel: String,
) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-slate-800 bg-slate-900 p-5">
            <p class="text-3xl font-semibold text-white">{value}</p>
            <p class="mt-1 text-sm text-slate-400">{label}</p>
            <Show when={
                let s = sublabel.clone();
                move || !s.is_empty()
            }>
                <p class="mt-1 text-xs text-slate-500">{sublabel.clone()}</p>
            </Show>
        </div>
    }
}

/// A horizontal bar list: one labelled row per slice, bar width proportional to
/// the largest value. Good for categorical breakdowns (matter types, workload).
#[component]
pub fn HBarList(data: Vec<MetricSlice>) -> impl IntoView {
    let max = data
        .iter()
        .map(|s| s.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    if data.is_empty() {
        return view! { <EmptyChart /> }.into_any();
    }
    view! {
        <div class="space-y-2.5">
            {data
                .into_iter()
                .enumerate()
                .map(|(i, s)| {
                    let pct = (s.value / max * 100.0).clamp(0.0, 100.0);
                    let bar_class = format!(
                        "h-2.5 rounded-full bg-current {}",
                        color_for(&s, i),
                    );
                    view! {
                        <div>
                            <div class="mb-1 flex items-center justify-between text-xs">
                                <span class="text-slate-300">{s.label.clone()}</span>
                                <span class="font-medium text-slate-400">{fmt_num(s.value)}</span>
                            </div>
                            <div class="h-2.5 w-full overflow-hidden rounded-full bg-slate-800">
                                <div class=bar_class style=format!("width:{pct:.1}%")></div>
                            </div>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
    .into_any()
}

/// A donut chart with an inline legend. Segments are drawn as dash-array arcs on
/// concentric SVG circles; colours come from each slice (or the palette).
#[component]
pub fn Donut(data: Vec<MetricSlice>) -> impl IntoView {
    let total: f64 = data.iter().map(|s| s.value).sum();
    if total <= 0.0 {
        return view! { <EmptyChart /> }.into_any();
    }

    // Geometry: r=60 in a 160x160 viewBox, stroke width 24.
    let radius = 60.0_f64;
    let circumference = 2.0 * std::f64::consts::PI * radius;

    let mut offset = 0.0_f64;
    let segments = data
        .iter()
        .enumerate()
        .filter(|(_, s)| s.value > 0.0)
        .map(|(i, s)| {
            let frac = s.value / total;
            let seg_len = frac * circumference;
            let dasharray = format!("{seg_len:.3} {:.3}", circumference - seg_len);
            let dashoffset = format!("{:.3}", -offset);
            offset += seg_len;
            let class = color_for(s, i);
            view! {
                <circle
                    cx="80"
                    cy="80"
                    r="60"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="24"
                    class=class
                    stroke-dasharray=dasharray
                    stroke-dashoffset=dashoffset
                    transform="rotate(-90 80 80)"
                />
            }
        })
        .collect_view();

    let legend = data
        .iter()
        .enumerate()
        .filter(|(_, s)| s.value > 0.0)
        .map(|(i, s)| {
            let pct = s.value / total * 100.0;
            let swatch = format!("h-2.5 w-2.5 rounded-full bg-current {}", color_for(s, i));
            view! {
                <div class="flex items-center justify-between gap-2 text-xs">
                    <span class="flex items-center gap-2 text-slate-300">
                        <span class=swatch></span>
                        {s.label.clone()}
                    </span>
                    <span class="text-slate-400">
                        {fmt_num(s.value)} " (" {format!("{pct:.0}%")} ")"
                    </span>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="flex flex-col items-center gap-5 sm:flex-row">
            <svg viewBox="0 0 160 160" class="h-40 w-40 shrink-0">
                <circle
                    cx="80"
                    cy="80"
                    r="60"
                    fill="none"
                    stroke-width="24"
                    class="text-slate-800"
                    stroke="currentColor"
                />
                {segments}
                <text
                    x="80"
                    y="80"
                    text-anchor="middle"
                    dominant-baseline="central"
                    class="fill-white text-2xl font-semibold"
                    style="font-size:1.75rem"
                >
                    {fmt_num(total)}
                </text>
            </svg>
            <div class="w-full flex-1 space-y-1.5">{legend}</div>
        </div>
    }
    .into_any()
}

/// A trend line chart (SVG polyline) with point markers and x-axis labels.
#[component]
pub fn TrendLine(data: Vec<MetricSlice>) -> impl IntoView {
    if data.len() < 2 {
        return view! { <EmptyChart /> }.into_any();
    }

    let width = 100.0_f64;
    let height = 40.0_f64;
    let max = data
        .iter()
        .map(|s| s.value)
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let n = data.len();
    let step = width / (n as f64 - 1.0);

    let points: Vec<(f64, f64)> = data
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let x = i as f64 * step;
            let y = height - (s.value / max * (height - 4.0)) - 2.0;
            (x, y)
        })
        .collect();

    let polyline = points
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");

    let markers = points
        .iter()
        .map(|(x, y)| {
            view! {
                <circle cx=format!("{x:.2}") cy=format!("{y:.2}") r="1.4" class="fill-primary-400" />
            }
        })
        .collect_view();

    let labels = data
        .iter()
        .map(|s| {
            view! {
                <span class="flex-1 text-center text-[10px] text-slate-500">{s.label.clone()}</span>
            }
        })
        .collect_view();

    view! {
        <div>
            <svg viewBox="0 0 100 40" preserveAspectRatio="none" class="h-32 w-full">
                <polyline
                    points=polyline
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    class="text-primary-400"
                    vector-effect="non-scaling-stroke"
                />
                {markers}
            </svg>
            <div class="mt-1 flex">{labels}</div>
        </div>
    }
    .into_any()
}

/// A labelled progress bar for a 0–100 rate.
#[component]
pub fn ProgressBar(
    #[prop(into)] label: String,
    percent: f64,
    #[prop(optional, default = "bg-emerald-500")] color: &'static str,
) -> impl IntoView {
    let pct = percent.clamp(0.0, 100.0);
    let bar_class = format!("h-2.5 rounded-full {color}");
    view! {
        <div>
            <div class="mb-1 flex items-center justify-between text-xs">
                <span class="text-slate-300">{label}</span>
                <span class="font-medium text-slate-400">{format!("{pct:.0}%")}</span>
            </div>
            <div class="h-2.5 w-full overflow-hidden rounded-full bg-slate-800">
                <div class=bar_class style=format!("width:{pct:.1}%")></div>
            </div>
        </div>
    }
}

/// Placeholder shown when a chart has no data to display.
#[component]
fn EmptyChart() -> impl IntoView {
    view! {
        <div class="rounded-lg border border-dashed border-slate-700 bg-slate-950/40 p-6 text-center text-xs text-slate-500">
            "Not enough data yet."
        </div>
    }
}
