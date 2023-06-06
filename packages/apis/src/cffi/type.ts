export abstract class Type {
  abstract readonly size: number;
  readonly align?: number;
}

export class Void extends Type {
  readonly size: number = 1;
}

export class Char extends Type {
  readonly size: number = 1;
}

export class UnsignedChar extends Type {
  readonly size: number = 1;
}

export class Short extends Type {
  readonly size: number = 2;
}

export class UnsignedShort extends Type {
  readonly size: number = 2;
}

export class Int extends Type {
  readonly size: number = 4;
}

export class UnsignedInt extends Type {
  readonly size: number = 4;
}

export class Long extends Type {
  readonly size: number = 8;
}

export class UnsignedLong extends Type {
  readonly size: number = 8;
}

export class Float extends Type {
  readonly size: number = 4;
}

export class Double extends Type {
  readonly size: number = 8;
}

export class USize extends Type {
  readonly size: number = 8;
}

export class I8 extends Type {
  readonly size: number = 1;
}

export class I16 extends Type {
  readonly size: number = 2;
}

export class I32 extends Type {
  readonly size: number = 4;
}

export class I64 extends Type {
  readonly size: number = 8;
}

export class U8 extends Type {
  readonly size: number = 1;
}

export class U16 extends Type {
  readonly size: number = 2;
}

export class U32 extends Type {
  readonly size: number = 4;
}

export class U64 extends Type {
  readonly size: number = 8;
}

export class F32 extends Type {
  readonly size: number = 4;
}

export class F64 extends Type {
  readonly size: number = 8;
}

export class Point<Item extends Type> extends Type {
  readonly size: number = 8;
  constructor(readonly itemType: Item) {
    super();
  }
}

export class Array<Item extends Type> extends Type {
  readonly size: number;
  readonly align: number;
  constructor(readonly itemType: Item, readonly length: number) {
    super();
    this.size = itemType.size * length;
    this.align = itemType.align || itemType.size;
  }
}

export class Struct<Fields extends { [filedName: string]: Type }> extends Type {
  readonly size: number;
  readonly align: number;
  readonly layout: {
    [key: string]: { offset: number; type: Type };
  };

  constructor(readonly fields: Fields, defaultAlgin: number = 8) {
    super();

    let offset = 0;
    this.align = 0;
    this.layout = {};

    for (const [fieldName, fieldType] of Object.entries(fields)) {
      const fieldSize = fieldType.size;
      const align = Math.min(fieldType.align || fieldType.size, defaultAlgin);

      this.align = Math.max(this.align, align);
      offset = Math.ceil(offset / align) * align;
      this.layout[fieldName] = { offset, type: fieldType };
      offset += fieldSize;
    }

    this.size = Math.ceil(offset / this.align) * this.align;
  }
}
