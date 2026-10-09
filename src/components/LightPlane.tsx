/**
 * Afterglow's signature: an angled plane of grainy light on void, ramping
 * ember → orchid → violet-soft → cobalt. Pure decoration, so it is hidden from
 * assistive tech and sits behind content.
 */
export default function LightPlane({ className = "", start = 38 }: { className?: string; start?: number }) {
  // `start` is where the plane's lit edge begins, in % of the width; keep
  // text to the left of it.
  return (
    <div className={`ag-light ag-grain ${className}`} aria-hidden="true">
      <div
        className="ag-light__haze ag-light__haze--ember"
        style={{ width: "34%", height: "70%", right: "-6%", top: "-10%" }}
      />
      <div
        className="ag-light__haze ag-light__haze--orchid"
        style={{ width: "22%", height: "80%", left: `${start + 10}%`, top: "30%" }}
      />
      <div
        className="ag-light__haze ag-light__haze--cobalt"
        style={{ width: "24%", height: "70%", left: "8%", bottom: "-30%" }}
      />
      <div className="ag-light__plane" style={{ left: `${start}%`, top: "-8%", width: `${100 - start}%`, height: "120%" }} />
    </div>
  );
}
