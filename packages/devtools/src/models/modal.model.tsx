import { StateModel } from "@bramblex/state-model";
import { ComponentType } from "react";
import { createPromise, uuid } from "../common/utils";
import { AppModel } from "./app.model";
import {
  AlertModal,
  ConfirmModal,
  NativeModal,
  ProgressModal,
} from "../modals";

export interface ModalComponentProps {
  close: () => any;
}

type ModalItem = {
  id: string;
  Component: ComponentType;
};

export type ModalModelState = ModalItem[];

export class ModalModel extends StateModel<ModalModelState> {
  constructor(public readonly app: AppModel) {
    super([]);
  }

  show<Props extends {}>(
    Component: ComponentType<Props & ModalComponentProps>,
    props: Props
  ) {
    const id = uuid();
    const close = () => {
      this.setState(this.state.filter(({ id: _id }) => _id !== id));
    };
    this.setState([
      ...this.state,
      {
        id,
        Component: () => <Component {...props} close={close} />,
      },
    ]);
    return close;
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

  progress(title: string): [ProgressModel, () => void] {
    const progress = new ProgressModel();
    return [progress, this.show(ProgressModal, { title, progress })];
  }
}

export class ProgressModel extends StateModel<{
  text: string;
  progress: number;
  isError: boolean
}> {
  private tasks: [string, () => Promise<void>][] = [];

  constructor() {
    super({ text: "", progress: 0, isError: false });
  }

  addTask(text: string, task: () => Promise<void>) {
    this.tasks.push([text, task]);
  }

  async run() {
    for (let i = 0, l = this.tasks.length; i < l; i++) {
      const [text, task] = this.tasks[i];
      this.setState({
        ...this.state,
        text: `(${i + 1}/${l})${text}`,
        progress: i / l,
      });
      try {
        await task();
        await new Promise((resolve) => setTimeout(resolve, 100));
      } catch (e) {
        this.setState({
          ...this.state,
          text: (e as any).toString(),
          isError: true
        });
        throw e;
      }
    }
    this.setState({ ...this.state, progress: 1 });
    await new Promise((resolve) => setTimeout(resolve, 300));
  }
}
