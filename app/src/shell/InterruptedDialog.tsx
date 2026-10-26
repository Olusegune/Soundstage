import { useRef } from "react";
import { useDialog } from "../ui";

export interface Interrupted {
  id: string;
  total: number;
  done: number;
  failed: number;
  remaining: string[];
  kept_takes: number;
}

/** Shown when a project is opened and a render run was cut short last time. */
export function InterruptedDialog(props: { job: Interrupted; onFinish: () => void; onKeep: () => void }) {
  const { job } = props;
  const first = useRef<HTMLButtonElement>(null);
  const ref = useDialog(props.onKeep, first);
  const left = job.remaining.length;
  return (
    <div className="scrim">
      <div className="dialog" role="alertdialog" aria-modal="true" aria-label="Rendering was interrupted" ref={ref}>
        <h2>Rendering was interrupted</h2>
        <div style={{ marginBottom: 20 }} className="muted">
          <p>
            The last render stopped before it finished: {job.done} of {job.total} line{job.total === 1 ? " was" : "s were"} done.
            {job.kept_takes > 0 ? ` Soundstage kept the ${job.kept_takes} take${job.kept_takes === 1 ? "" : "s"} it had made, so you are not charged for them again.` : ""}
          </p>
          {left > 0 && <p>{left} line{left === 1 ? " is" : "s are"} left.</p>}
        </div>
        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button ref={first} onClick={props.onKeep}>Keep what was done</button>
          {left > 0 && <button className="primary" onClick={props.onFinish}>Finish the other {left}</button>}
        </div>
      </div>
    </div>
  );
}
