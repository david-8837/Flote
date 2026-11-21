import { useId, type ReactNode } from "react";

interface CardProps {
  /** Heading above the group, as macOS System Settings labels its groups. */
  title?: string;
  children: ReactNode;
}

export function Card({ title, children }: CardProps) {
  if (!title) return <section className="card">{children}</section>;
  return (
    <section className="card-group" aria-label={title}>
      <h2 className="card-title">{title}</h2>
      <div className="card">{children}</div>
    </section>
  );
}

interface RowProps {
  label: string;
  hint?: string;
  icon?: ReactNode;
  stacked?: boolean;
  children: ReactNode;
}

export function Row({ label, hint, icon, stacked, children }: RowProps) {
  return (
    <div className={stacked ? "row row-stacked" : "row"}>
      {icon ? <span className="row-icon">{icon}</span> : null}
      <div className="row-text">
        <span className="row-label">{label}</span>
        {hint ? <span className="row-hint">{hint}</span> : null}
      </div>
      <div className="row-control">{children}</div>
    </div>
  );
}

interface SubRowProps extends Omit<RowProps, "icon"> {
  /** Whether the option it depends on is on; the row folds away while it is off. */
  open: boolean;
}

/**
 * An option that only makes sense while the row above it is on. It stays
 * mounted so it can fold open and shut, and is `inert` while folded so the
 * keyboard and screen readers skip it.
 */
export function SubRow({ open, ...row }: SubRowProps) {
  return (
    <div className="row-sub" data-open={open} inert={!open}>
      <div className="row-sub-clip">
        <Row {...row} />
      </div>
    </div>
  );
}

interface ToggleProps {
  checked: boolean;
  label: string;
  onChange: (checked: boolean) => void;
}

export function Toggle({ checked, label, onChange }: ToggleProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      className="toggle"
      onClick={() => onChange(!checked)}
    >
      <span className="toggle-knob" />
    </button>
  );
}

interface SliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  label: string;
  unit?: string;
  displayValue?: string;
  disabled?: boolean;
  onChange: (value: number) => void;
}

export function Slider({
  value,
  min,
  max,
  step = 1,
  label,
  unit = " px",
  displayValue,
  disabled,
  onChange,
}: SliderProps) {
  const filled = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));
  const text = displayValue ?? `${value}${unit}`;
  return (
    <div className="slider">
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={value}
        aria-valuetext={text}
        disabled={disabled}
        style={{ "--filled": `${filled}%` } as React.CSSProperties}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <output className="slider-value" aria-hidden="true">{text}</output>
    </div>
  );
}

interface VisualChoicesProps<T extends string> {
  label: string;
  value: T;
  hideLabel?: boolean;
  options: readonly { value: T; label: string; preview: ReactNode; detail?: string }[];
  onChange: (value: T) => void;
}

/** Native radios provide arrow-key navigation without extra listeners or state. */
export function VisualChoices<T extends string>({ label, value, hideLabel, options, onChange }: VisualChoicesProps<T>) {
  const name = useId();
  return (
    <fieldset className="visual-setting">
      <legend className={hideLabel ? "sr-only" : "visual-legend"}>{label}</legend>
      <div className="visual-choices" style={{ gridTemplateColumns: `repeat(${options.length}, minmax(0, 1fr))` }}>
        {options.map((option) => (
          <label className="visual-choice" key={option.value}>
            <input type="radio" name={name} value={option.value} checked={value === option.value}
              onChange={() => onChange(option.value)} />
            <span className="visual-choice-body">
              <span className="visual-preview" aria-hidden="true">{option.preview}</span>
              <span className="visual-choice-label">{option.label}
                <span className="visual-check" aria-hidden="true">
                <svg viewBox="0 0 16 16" width="12" height="12"><path d="m4 8 2.5 2.5L12 5" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg>
                </span>
              </span>
              {option.detail ? <span className="visual-choice-detail">{option.detail}</span> : null}
            </span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

interface SegmentedProps<T extends string> {
  value: T;
  label: string;
  options: readonly { value: T; label: string }[];
  onChange: (value: T) => void;
}

export function Segmented<T extends string>({ value, label, options, onChange }: SegmentedProps<T>) {
  const name = useId();
  return (
    <div className="segmented" role="radiogroup" aria-label={label}>
      {options.map((option) => (
        <label key={option.value} className="segment">
          <input type="radio" className="sr-only" name={name} value={option.value}
            checked={option.value === value} onChange={() => onChange(option.value)} />
          <span>{option.label}</span>
        </label>
      ))}
    </div>
  );
}
