export type NativeType =
  | "char"
  | "unsigned char"
  | "short"
  | "unsigned short"
  | "int"
  | "unsigned int"
  | "long"
  | "unsigned long"
  | "float"
  | "double"
  | "long int"
  | "long long"
  | "long double";

export type StandardType =
  | "i8"
  | "i16"
  | "i32"
  | "i64"
  | "i128"
  | "u8"
  | "u16"
  | "u32"
  | "u64"
  | "u128"
  | "f32"
  | "f64"
  | "f128";

export const StandardTypeSize: { [key in StandardType]: number } = {
  i8: 1,
  i16: 2,
  i32: 4,
  i64: 8,
  i128: 16,
  u8: 1,
  u16: 2,
  u32: 4,
  u64: 8,
  u128: 16,
  f32: 4,
  f64: 8,
  f128: 16,
};

// x64
export const NativeTypeMap: { [key in NativeType]: StandardType } = {
  char: "i8",
  "unsigned char": "u8",
  short: "i16",
  "unsigned short": "u16",
  int: "i32",
  "unsigned int": "u32",
  long: "i64",
  "unsigned long": "u64",
  float: "f32",
  double: "f64",
  "long int": "i64",
  "long long": "i64",
  "long double": "f128",
};

const nativeTypeSizeMap = {
	char: 1,
	"unsigned char": 1,
	short: 2,
	"unsigned short": 2,
	int: 4,
	"unsigned int": 4,
	long: 8,
	"unsigned long": 8,
	float: 4,
	double: 8,
	"long int": 8,
	"long long": 8,
	"long double": 16,
};

class ArrayType {
	constructor(elementType, length) {
	}
}

class StructType {
	constructor(declare, defaultAlign = 8) {
		this.declare = declare;
		this.layout = {};
		this.maxAlign = 0;

		let offset = 0;
		for (const [fieldName, fieldType] of Object.entries(declare)) {
			let fieldSize, align;

			if (typeof fieldType === 'string') {
				fieldSize = nativeTypeSizeMap[fieldType];
				align = Math.min(fieldSize, defaultAlign);
			} else if (fieldType instanceof StructType) {
				fieldSize = fieldType.size;
				align = fieldType.maxAlign;
			} else {
				throw new Error(`Unknown field type: ${fieldType}`);
			}


			this.maxAlign = Math.max(this.maxAlign, align);
			offset = Math.ceil(offset / align) * align;
			this.layout[fieldName] = [offset, fieldSize];
			offset += fieldSize;
		}

		this.size = Math.ceil(offset / this.maxAlign) * this.maxAlign;
	}

	show() {
		console.dir(this, { depth: null });
	}
}

// new StructType({
// 	c1: 'char',
// 	i: 'int',
// 	c2: 'char',
// }).show();

// new StructType({
// 	d: 'double',
// 	c: 'char',
// 	i: 'int',
// }).show();

new StructType({
	c1: 'char',
	s3: new StructType({
		d: 'double',
		c: 'char',
		i: 'int',
	}),
	d: 'double'
}).show();
