import type { CSSProperties, ReactNode } from "react";
import {
  Bar,
  BarChart as RBarChart,
  CartesianGrid,
  Legend,
  Line,
  LineChart as RLineChart,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { useRowFilter } from "./filter";

type Tone = "success" | "danger" | "warning" | "info" | "neutral";

const toneVar: Record<Tone, string> = {
  success: "var(--ok)",
  danger: "var(--bad)",
  warning: "var(--warn)",
  info: "var(--info)",
  neutral: "var(--muted)",
};

const chartColors = ["var(--accent)", "var(--info)", "var(--warn)", "var(--ok)"];

export function Stack({
  gap = 12,
  children,
  style,
}: {
  gap?: number;
  children?: ReactNode;
  style?: CSSProperties;
}) {
  return (
    <div className="stack" style={{ gap, ...style }}>
      {children}
    </div>
  );
}

export function Row({
  gap = 12,
  children,
}: {
  gap?: number;
  children?: ReactNode;
}) {
  return (
    <div className="row" style={{ gap }}>
      {children}
    </div>
  );
}

export function Grid({
  columns = 2,
  gap = 16,
  children,
}: {
  columns?: number;
  gap?: number;
  children?: ReactNode;
}) {
  return (
    <div
      className="grid"
      style={{ gap, gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` }}
    >
      {children}
    </div>
  );
}

export function H1({ children }: { children?: ReactNode }) {
  return <h1 className="h1">{children}</h1>;
}

export function H2({ children }: { children?: ReactNode }) {
  return <h2 className="h2">{children}</h2>;
}

export function Text({
  children,
  tone = "primary",
  size = "md",
  weight = "normal",
  as,
}: {
  children?: ReactNode;
  tone?: "primary" | "secondary" | "tertiary";
  size?: "small" | "md";
  weight?: "normal" | "semibold";
  as?: "span";
}) {
  const Tag = as === "span" ? "span" : "p";
  return (
    <Tag className={`text text-${tone} text-${size} text-${weight}`}>{children}</Tag>
  );
}

export function Stat({
  value,
  label,
  tone = "neutral",
}: {
  value: string;
  label: string;
  tone?: Tone;
}) {
  return (
    <div className="stat">
      <div className="stat-value" style={{ color: toneVar[tone] }}>
        {value}
      </div>
      <div className="stat-label">{label}</div>
    </div>
  );
}

export function Pill({
  children,
  size = "md",
}: {
  children?: ReactNode;
  size?: "sm" | "md";
}) {
  return <span className={`pill pill-${size}`}>{children}</span>;
}

export function Card({ children }: { children?: ReactNode }) {
  return <div className="card">{children}</div>;
}

export function CardHeader({
  children,
  trailing,
}: {
  children?: ReactNode;
  trailing?: ReactNode;
}) {
  return (
    <div className="card-header">
      <div>{children}</div>
      {trailing}
    </div>
  );
}

export function CardBody({ children }: { children?: ReactNode }) {
  return <div className="card-body">{children}</div>;
}

export function Callout({
  title,
  children,
  tone = "info",
}: {
  title: string;
  children?: ReactNode;
  tone?: Tone;
}) {
  return (
    <aside className={`callout callout-${tone}`}>
      <strong>{title}</strong>
      <div className="callout-body">{children}</div>
    </aside>
  );
}

export function Table({
  headers,
  rows,
  rowTone,
  columnAlign,
  striped,
}: {
  headers: string[];
  rows: (string | number)[][];
  rowTone?: Tone[];
  columnAlign?: ("left" | "right" | "center")[];
  striped?: boolean;
}) {
  const filter = useRowFilter();
  const visible = rows
    .map((row, ri) => ({ row, ri, tone: rowTone?.[ri] }))
    .filter(({ row }) =>
      !filter || String(row[0] ?? "").toLowerCase().includes(filter),
    );

  return (
    <div className="table-wrap">
      <table className={striped ? "table striped" : "table"}>
        <thead>
          <tr>
            {headers.map((h, i) => (
              <th key={h} style={{ textAlign: columnAlign?.[i] ?? "left" }}>
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {visible.map(({ row, ri, tone }) => (
            <tr key={ri} className={tone ? `tone-${tone}` : undefined}>
              {row.map((cell, ci) => (
                <td key={ci} style={{ textAlign: columnAlign?.[ci] ?? "left" }}>
                  {cell}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {filter && visible.length === 0 ? (
        <p className="table-empty">无匹配 “{filter}” 的行</p>
      ) : null}
    </div>
  );
}

type Series = {
  name: string;
  data: number[];
  tone?: Tone;
};

export function BarChart({
  categories,
  series,
  height = 260,
  valueSuffix = "",
  yMax,
  showValues,
}: {
  categories: string[];
  series: Series[];
  height?: number;
  valueSuffix?: string;
  yMax?: number;
  showValues?: boolean;
}) {
  const data = categories.map((name, i) => {
    const row: Record<string, string | number> = { name };
    for (const s of series) row[s.name] = s.data[i] ?? 0;
    return row;
  });

  return (
    <div className="chart" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <RBarChart data={data} margin={{ top: 8, right: 8, left: 0, bottom: 48 }}>
          <CartesianGrid stroke="var(--grid)" vertical={false} />
          <XAxis
            dataKey="name"
            tick={{ fill: "var(--muted)", fontSize: 11 }}
            interval={0}
            angle={-28}
            textAnchor="end"
            height={60}
          />
          <YAxis
            tick={{ fill: "var(--muted)", fontSize: 11 }}
            domain={yMax != null ? [0, yMax] : [0, "auto"]}
          />
          <Tooltip
            formatter={(v: number) => `${v.toLocaleString()}${valueSuffix}`}
            contentStyle={{
              background: "var(--panel)",
              border: "1px solid var(--stroke)",
              borderRadius: 6,
            }}
          />
          <Legend />
          {series.map((s, i) => (
            <Bar
              key={s.name}
              dataKey={s.name}
              fill={chartColors[i % chartColors.length]}
              radius={[3, 3, 0, 0]}
              label={
                showValues
                  ? { position: "top", fill: "var(--muted)", fontSize: 10 }
                  : undefined
              }
            />
          ))}
        </RBarChart>
      </ResponsiveContainer>
    </div>
  );
}

export function LineChart({
  categories,
  series,
  height = 280,
  referenceLines,
  beginAtZero,
}: {
  categories: string[];
  series: { name: string; data: number[] }[];
  height?: number;
  referenceLines?: { value: number; label: string }[];
  beginAtZero?: boolean;
}) {
  const data = categories.map((name, i) => {
    const row: Record<string, string | number> = { name };
    for (const s of series) row[s.name] = s.data[i] ?? 0;
    return row;
  });

  return (
    <div className="chart" style={{ height }}>
      <ResponsiveContainer width="100%" height="100%">
        <RLineChart data={data} margin={{ top: 8, right: 16, left: 0, bottom: 8 }}>
          <CartesianGrid stroke="var(--grid)" />
          <XAxis dataKey="name" tick={{ fill: "var(--muted)", fontSize: 11 }} />
          <YAxis
            tick={{ fill: "var(--muted)", fontSize: 11 }}
            domain={beginAtZero ? [0, "auto"] : ["auto", "auto"]}
          />
          <Tooltip
            formatter={(v: number) => v.toLocaleString()}
            contentStyle={{
              background: "var(--panel)",
              border: "1px solid var(--stroke)",
              borderRadius: 6,
            }}
          />
          <Legend />
          {referenceLines?.map((rl) => (
            <ReferenceLine
              key={rl.label}
              y={rl.value}
              stroke="var(--bad)"
              strokeDasharray="4 4"
              label={{ value: rl.label, fill: "var(--bad)", fontSize: 11 }}
            />
          ))}
          {series.map((s, i) => (
            <Line
              key={s.name}
              type="monotone"
              dataKey={s.name}
              stroke={chartColors[i % chartColors.length]}
              strokeWidth={2}
              dot={{ r: 3 }}
              activeDot={{ r: 5 }}
            />
          ))}
        </RLineChart>
      </ResponsiveContainer>
    </div>
  );
}
