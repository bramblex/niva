import { useModel } from "@bramblex/state-model-react";
import classNames from "classnames";
import { XPromise } from "../common/utils";

import { ModalComponentProps, ProgressModel } from "../models/modal.model";

import "./style.scss";
import { useLocale, useModal } from "../models/app.model";

export function NativeModal(_: ModalComponentProps) {
  return <></>;
}

export function AlertModal(
  props: ModalComponentProps & {
    title: string;
    message: string;
    promise: XPromise<void>;
  }
) {
  const { close, title, message, promise } = props;
  const locale = useLocale();
  return (
    <div className="window active is-bright">
      <i
        className="icon-close"
        onClick={() => {
          close();
          promise.resolve();
        }}
      ></i>
      <div className="window-body has-space">
        <h4>{title}</h4>
        <p>{message}</p>
      </div>
      <footer style={{ textAlign: "right" }}>
        <button
          className="btn btn-md btn-primary"
          onClick={() => {
            close();
            promise.resolve();
          }}
        >
          {locale.t("CONFIRM")}
        </button>
      </footer>
    </div>
  );
}

export function ConfirmModal(
  props: ModalComponentProps & {
    title: string;
    message: string;
    promise: XPromise<boolean>;
  }
) {
  const { close, title, message, promise } = props;
  const locale = useLocale();
  return (
    <div className="window active is-bright">
      <i
        className="icon-close"
        onClick={() => {
          close();
          promise.resolve(false);
        }}
      ></i>
      <div className="window-body has-space">
        <h4>{title}</h4>
        <p>{message}</p>
      </div>
      <footer style={{ textAlign: "right" }}>
        <button
          className="btn btn-md"
          style={{ marginRight: "6px" }}
          onClick={() => {
            close();
            promise.resolve(false);
          }}
        >
          {locale.t("CANCEL")}
        </button>
        <button
          className="btn btn-md btn-primary"
          onClick={() => {
            close();
            promise.resolve(true);
          }}
        >
          {locale.t("CONFIRM")}
        </button>
      </footer>
    </div>
  );
}

export function ProgressModal({
  close,
  title,
  progress,
}: ModalComponentProps & { title: string; progress: ProgressModel }) {
  useModel(progress);
  const { state } = progress;
  const locale = useLocale();
  const { isError } = progress.state
  return (
    <div className="window active is-bright">
      <div className="window-body has-space progress">
        <h4 className="instruction instruction-primary">{isError ? '构建失败' : title}</h4>

        {state.progress > 0 ? (
          <>
            <p>已完成{Math.floor(state.progress * 100) + "%"}</p>
            <div role={"progressbar"} className="progressbar">
              <div
                style={{ width: state.progress * 100 + "%" }}
                className="progressbar-animated"
              ></div>
            </div>
          </>
        ) : (
          <div role="progressbar" className="marquee"></div>
        )}
        <p>{state.text}</p>
      </div>

      {
        isError && <button onClick={close}
          className="btn btn-bg btn-primary btn-modal"
        >
          {locale.t('CANCEL')}
        </button>
      }
    </div>
  );
}

export function Modal() {
  const modal = useModal();
  const modals = modal.state;

  return modals.length > 0 ? (
    <div className="modal-container">
      {modals.map(({ id, Component }, i) => (
        <div
          key={id}
          className={classNames("modal", {
            "modal-active": modals.length === i + 1,
          })}
        >
          <Component />
        </div>
      ))}
    </div>
  ) : null;
}
