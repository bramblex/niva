import type {} from "@niva/types/node";

type Equal<Left, Right> =
  (<Value>() => Value extends Left ? 1 : 2) extends (<Value>() => Value extends Right ? 1 : 2)
    ? (<Value>() => Value extends Right ? 1 : 2) extends (<Value>() => Value extends Left ? 1 : 2)
      ? true
      : false
    : false;
type Expect<Condition extends true> = Condition;

type FsReadFileMatchesNode = Expect<Equal<typeof Niva.fs.readFile, typeof import("node:fs").readFile>>;
type FsPromiseReadFileMatchesNode = Expect<Equal<typeof Niva.fs.promises.readFile, typeof import("node:fs/promises").readFile>>;
type SpawnMatchesNode = Expect<Equal<typeof Niva.child_process.spawn, typeof import("node:child_process").spawn>>;
type BufferMatchesNode = Expect<Equal<typeof Niva.buffer.Buffer, typeof import("node:buffer").Buffer>>;
type StreamMatchesNode = Expect<Equal<typeof Niva.stream.Stream, typeof import("node:stream").Stream>>;
type ReadableFromWebMatchesNode = Expect<Equal<typeof Niva.stream.Readable.fromWeb, typeof import("node:stream").Readable.fromWeb>>;

const fileBuffer: Buffer = Niva.buffer.Buffer.from("native Node tooling");
const currentDirectory: string = process.cwd();
const processEnv: NodeJS.ProcessEnv = process.env;

void fileBuffer;
void currentDirectory;
void processEnv;
type _FsReadFileMatchesNode = FsReadFileMatchesNode;
type _FsPromiseReadFileMatchesNode = FsPromiseReadFileMatchesNode;
type _SpawnMatchesNode = SpawnMatchesNode;
type _BufferMatchesNode = BufferMatchesNode;
type _StreamMatchesNode = StreamMatchesNode;
type _ReadableFromWebMatchesNode = ReadableFromWebMatchesNode;
