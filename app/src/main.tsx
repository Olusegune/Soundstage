import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

class Boundary extends React.Component<{ children: React.ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() { return { failed: true }; }
  componentDidCatch(error: unknown) { console.error("UI error", error); }
  render() {
    if (this.state.failed) {
      return (
        <div style={{ padding: 40 }}>
          <h1>Something went wrong</h1>
          <p>Your project was saved automatically. Restart Soundstage to continue.</p>
        </div>
      );
    }
    return this.props.children;
  }
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode><Boundary><App /></Boundary></React.StrictMode>,
);
