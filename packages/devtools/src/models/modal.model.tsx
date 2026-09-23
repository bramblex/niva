import { StateModel } from "@bramblex/state-model";
import { ComponentType } from "react";
import { createPromise, uuid } from "../common/utils";
import { AppModel } from "./app.model";
import {
  AlertModal,
  ConfirmModal,
  NativeModal,
  ProgressModal,
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

  progress(title: string): [ProgressModel, () => void] {
    const progress = new ProgressModel();
    return [progress, this.show(ProgressModal, { title, progress })];
  }
}

export class ProgressModel extends StateModel<{
  text: string;
  step: number;
  total: number;
  commands: BuildCommand[];
  stdinOpen: boolean;
}> {
  private tasks: [string, () => Promise<void>][] = [];
  private activeStream: ReturnType<typeof Niva.stream> | null = null;
  private readonly outputLimit = 250_000;

  constructor() {
    super({ text: "", step: 0, total: 0, commands: [], stdinOpen: false });
  }

  addTask(text: string, task: () => Promise<void>) {
    this.tasks.push([text, task]);
  }

  async run() {
    for (let i = 0, l = this.tasks.length; i < l; i++) {
      const [text, task] = this.tasks[i];
      this.update({
        ...this.state,
        text,
        step: i + 1,
        total: l,
      });
      try {
        await task();
        await new Promise((resolve) => setTimeout(resolve, 100));
      } catch (e) {
        this.update({
          ...this.state,
          text: (e as any).toString(),
        });
        throw e;
      }
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }

  async runCommand(
    label: string,
    cmd: string,
    args: string[],
    options?: { env?: Record<string, string>; currentDir?: string }
  ): Promise<void> {
    const commandIndex = this.state.commands.length;
    const command: BuildCommand = {
      label,
      transcript: [],
      status: "running",
      exitCode: null,
    };
    this.update({
      ...this.state,
      commands: [...this.state.commands, command],
      stdinOpen: true,
    });

    const decoders = [new TextDecoder(), new TextDecoder()];
    const sawChunks = [false, false];
    const outputJobs: Promise<void>[] = [];
    const outputEnded = [false, false];
    const outputEndResolvers: Array<() => void> = [];
    const outputEndSignals = [0, 1].map(
      (index) => new Promise<void>((resolve) => { outputEndResolvers[index] = resolve; })
    );
    const appendDecoded = (text: string, isStderr: boolean) => {
      if (text) this.appendCommandOutput(commandIndex, text, isStderr);
    };
    const streamWithChunks = Niva.stream as unknown as (
      method: string,
      args: any[],
      handlers: {
        onChunk?: (bytes: Uint8Array, isStderr: boolean) => void;
        onBlob?: (blob: Blob, isStderr: boolean) => void;
      }
    ) => ReturnType<typeof Niva.stream>;

    let stream: ReturnType<typeof Niva.stream> | null = null;
    try {
      stream = streamWithChunks(
        "process.execStream",
        [cmd, args, options ?? null],
        {
          // onChunk keeps the terminal live. onBlob remains a compatibility
          // path for runtimes whose bridge has not exposed per-frame chunks.
          onChunk: (bytes, isStderr) => {
            const index = isStderr ? 1 : 0;
            sawChunks[index] = true;
            appendDecoded(decoders[index].decode(bytes, { stream: true }), isStderr);
          },
          onBlob: (blob, isStderr) => {
            const index = isStderr ? 1 : 0;
            if (!outputEnded[index]) {
              outputEnded[index] = true;
              outputEndResolvers[index]();
            }
            const output = sawChunks[index]
              ? Promise.resolve()
              : blob.text().then((text) => appendDecoded(text, isStderr));
            outputJobs.push(output);
          },
        }
      );
      this.activeStream = stream;
      const result = await stream.promise as { status: number | null };
      // The final result can race the OS pipe pumps. END on both substreams
      // confirms their last chunks reached the renderer before closing logs.
      await Promise.race([
        Promise.all(outputEndSignals),
        new Promise<void>((resolve) => setTimeout(resolve, 1500)),
      ]);
      await Promise.all(outputJobs);
      for (let index = 0; index < decoders.length; index++) {
        appendDecoded(decoders[index].decode(), index === 1);
      }

      this.updateCommand(commandIndex, {
        status: result.status === 0 ? "success" : "failed",
        exitCode: result.status,
      });
      if (result.status !== 0) {
        throw new Error(`${label}: ${result.status ?? "unknown exit status"}`);
      }
    } catch (error) {
      await Promise.all(outputJobs);
      this.updateCommand(commandIndex, { status: "failed" });
      throw error;
    } finally {
      if (this.activeStream?.id === stream?.id) this.activeStream = null;
      this.update({ ...this.state, stdinOpen: false });
    }
  }

  sendInput(line: string): boolean {
    const stream = this.activeStream;
    if (!stream || !this.state.stdinOpen) return false;
    const accepted = Niva.streamSend(stream.id, `${line}\n`);
    if (accepted) {
      const commandIndex = this.state.commands.length - 1;
      this.appendCommandInput(commandIndex, line);
    }
    return accepted;
  }

  closeStdin(): boolean {
    const stream = this.activeStream;
    if (!stream || !this.state.stdinOpen) return false;
    const accepted = Niva.streamSend(stream.id, new Uint8Array(0), true);
    if (accepted) this.update({ ...this.state, stdinOpen: false });
    return accepted;
  }

  private appendCommandOutput(index: number, text: string, isStderr: boolean) {
    const command = this.state.commands[index];
    if (!command) return;
    const current = command.transcript;
    const kind: BuildTranscriptEntry["kind"] = isStderr ? "stderr" : "stdout";
    const last = current[current.length - 1];
    const transcript = last?.kind === kind
      ? [...current.slice(0, -1), { kind, text: last.text + text }]
      : [...current, { kind, text }];
    this.updateCommand(index, { transcript: this.trimTranscript(transcript) });
  }

  private appendCommandInput(index: number, line: string) {
    const command = this.state.commands[index];
    if (!command) return;
    this.updateCommand(index, {
      transcript: [...command.transcript, { kind: "stdin", text: `> ${line}\n` }],
    });
  }

  private trimTranscript(transcript: BuildTranscriptEntry[]): BuildTranscriptEntry[] {
    const outputLength = transcript.reduce(
      (sum, entry) => sum + (entry.kind === "stdin" ? 0 : entry.text.length),
      0
    );
    let toTrim = outputLength - this.outputLimit;
    if (toTrim <= 0) return transcript;

    const trimmed = transcript.slice();
    while (toTrim > 0 && trimmed.length > 0) {
      const first = trimmed[0];
      if (first.kind === "stdin") {
        trimmed.shift();
        continue;
      }
      if (first.text.length <= toTrim) {
        toTrim -= first.text.length;
        trimmed.shift();
      } else {
        trimmed[0] = {
          ...first,
          text: `[earlier output truncated]\n${first.text.slice(toTrim)}`,
        };
        toTrim = 0;
      }
    }
    return trimmed;
  }

  private updateCommand(index: number, patch: Partial<BuildCommand>) {
    const commands = this.state.commands.slice();
    const command = commands[index];
    if (!command) return;
    commands[index] = { ...command, ...patch };
    this.update({ ...this.state, commands });
  }
}

export interface BuildCommand {
  label: string;
  transcript: BuildTranscriptEntry[];
  status: "running" | "success" | "failed";
  exitCode: number | null;
}

export interface BuildTranscriptEntry {
  kind: "stdout" | "stderr" | "stdin";
  text: string;
}
