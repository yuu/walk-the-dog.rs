import type { MetaFunction } from "@remix-run/node";
import { useEffect, useRef } from "react";
import init from "walk-the-dog-core";

export const meta: MetaFunction = () => {
  return [
    { title: "New Remix App" },
    { name: "description", content: "Welcome to Remix!" },
  ];
};

export default function Index() {
  const initialized = useRef<boolean>(false);

  useEffect(() => {
    if (!initialized.current) {
      init();
      initialized.current = true;
    }
  }, [initialized]);

  return (
    <div className="font-sans p-4">
      <h1 className="text-3xl">Welcome to Remix</h1>
      <canvas id="canvas" width={1200} height={600} />
    </div>
  );
}
