import React from "react";
import ReactDOM from "react-dom/client";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import App from "./App";
import "./index.css";

import enCommon from "./locales/en/common.json";
import zhCommon from "./locales/zh-CN/common.json";
import frCommon from "./locales/fr/common.json";

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: enCommon },
      "zh-CN": { translation: zhCommon },
      fr: { translation: frCommon },
    },
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
