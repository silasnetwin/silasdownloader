import { useState, type ReactNode } from "react";
import { ChevronDown } from "lucide-react";

interface SectionProps {
  /** Die Überschrift des Abschnitts. */
  title: string;
  /** Ein kurzer Hinweis darunter. */
  subtitle?: string;
  /** Ein Symbol links neben der Überschrift. */
  icon?: ReactNode;
  /** Eine Aktion auf der rechten Seite, zum Beispiel „Sperren". */
  action?: ReactNode;
  /** Ob der Abschnitt beim Öffnen ausgeklappt ist. */
  defaultOpen?: boolean;
  /** Eine zusätzliche Klasse, zum Beispiel für eine Warnung. */
  tone?: string;
  children: ReactNode;
}

/**
 * Ein aufklappbarer Abschnitt.
 *
 * Die Einstellungen der App sind in Abschnitte sortiert. Jeder Abschnitt lässt
 * sich einklappen, damit nicht alles gleichzeitig offen ist.
 */
export default function Section({
  title,
  subtitle,
  icon,
  action,
  defaultOpen = true,
  tone,
  children,
}: SectionProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <section className={`card section${open ? " is-open" : ""}${tone ? ` ${tone}` : ""}`}>
      <div className="card-head">
        <button
          type="button"
          className="section-toggle"
          onClick={() => setOpen((value) => !value)}
          aria-expanded={open}
        >
          {icon && <span className="section-mark">{icon}</span>}
          <span className="section-text">
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
          </span>
          <ChevronDown size={16} className="section-chevron" />
        </button>
        {action}
      </div>
      {open && <div className="section-body">{children}</div>}
    </section>
  );
}
