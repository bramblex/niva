import { StateModel } from "@bramblex/state-model";
import { ComponentType } from "react";
import { createPromise, uuid } from "../common/utils";
import { AppModel } from "./app.model";
import {
  AlertModal,
  ConfirmModal,
  NativeModal,
  UnsavedChangesModal,
  PromptProjectNameModal,
} from "../modals";

export interface ModalComponentProps {
  close: () => any;
}

type ModalItem = {
  id: string;
  Component: ComponentType;
};

export type ModalModelState = {
  modals: ModalItem[];
};

export class ModalModel extends StateModel<ModalModelState> {
  constructor(public readonly app: AppModel) {
    super({ modals: [] });
  }

  show<Props extends {}>(
    Component: ComponentType<Props & ModalComponentProps>,
    props: Props
  ) {
    const id = uuid();
    const close = () => {
      this.update({
        modals: this.state.modals.filter(({ id: _id }) => _id !== id),
      });
    };
    this.update({
      modals: [
        ...this.state.modals,
        {
          id,
          Component: () => <Component {...props} close={close} />,
        },
      ],
    });
    return close;
  }

  destroyAll() {
    this.update({ modals: [] });
  }

  async showNative<T>(callback: () => Promise<T>): Promise<T | null> {
    const close = this.show(NativeModal, {});
    await new Promise((resolve) => setTimeout(resolve, 50));
    try {
      return await callback();
    } catch (err) {
      return null;
    } finally {
      close();
    }
  }

  alert(title: string, message: string): Promise<void> {
    const promise = createPromise<void>();
    this.show(AlertModal, {
      title,
      message,
      promise,
    });
    return promise;
  }

  confirm(title: string, message: string): Promise<boolean> {
    const promise = createPromise<boolean>();
    this.show(ConfirmModal, {
      title,
      message,
      promise,
    });
    return promise;
  }

  confirmUnsaved(): Promise<"save" | "discard" | "cancel"> {
    const promise = createPromise<"save" | "discard" | "cancel">();
    this.show(UnsavedChangesModal, { promise });
    return promise;
  }

  promptProjectName(): Promise<string | null> {
    const promise = createPromise<string | null>();
    this.show(PromptProjectNameModal, { promise });
    return promise;
  }

}
