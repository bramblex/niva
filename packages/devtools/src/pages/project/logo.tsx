import { useState } from "react";
import defaultLogo from "../../assets/logo-default.png";

export function Logo(props: { src: string | null }) {
  const [src, setSrc] = useState(props.src);

  return (
    <img
      style={{ height: "100%", width: "100%" }}
      alt="logo"
      src={src || defaultLogo}
      onError={() => {
        if (src !== defaultLogo) {
          setSrc(defaultLogo);
        }
      }}
    />
  );
}
