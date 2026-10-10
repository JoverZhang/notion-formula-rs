import * as Y from 'yjs';

import { FieldType } from '@/application/database-yjs/database.type';
import type { FormulaCellResult } from '@/application/database-yjs/fields/formula/formula.type';
import type { FormulaType, FormulaValue } from '@/application/database-yjs/fields/formula/values';
import {
  YDatabase,
  YDatabaseCell,
  YDatabaseCells,
  YDatabaseField,
  YDatabaseFields,
  YDatabaseRow,
  YDatabaseView,
  YDatabaseViews,
  YDoc,
  YjsDatabaseKey as K,
  YjsEditorKey as E,
} from '@/application/types';

import { Family, Operation, SetupOptions } from './types';

interface FieldSpec {
  id: string;
  type: FieldType;
  options?: Record<string, unknown>;
}

interface RowModel {
  index: number;
  base: number;
  a: number;
  b: number;
  c: number;
  label: string;
  tags: string[];
  price: number;
  quantity: number;
  discountEighths: number;
  taxSixteenths: number;
  cancelled: boolean;
  paid: boolean;
}

const tagNames = ['Blue', 'Green', 'Orange', 'Red'];
const input = (id: string, type = FieldType.Number): FieldSpec => ({ id, type });
const formula = (id: string, expression: string): FieldSpec => ({
  id,
  type: FieldType.Formula,
  options: { expression },
});

function fieldsFor(family: Family, formulaVersion: number): FieldSpec[] {
  const bias = formulaVersion * 7;

  switch (family) {
    case 'arithmetic':
      return [
        input('a'), input('b'), input('c'),
        formula('sum', 'prop("a") + prop("b")'),
        formula('score', `prop("a") * prop("b") + prop("c") + ${bias}`),
        formula('scaled', '(prop("a") - prop("b")) / 2 + prop("c")'),
      ];
    case 'text-list':
      return [
        input('label', FieldType.RichText),
        {
          ...input('tags', FieldType.MultiSelect),
          options: {
            content: JSON.stringify({
              options: tagNames.map((name, index) => ({ id: `tag-${index}`, name, color: 'Blue' })),
              disable_color: false,
            }),
          },
        },
        formula('normalized', `lower(trim(prop("label"))) + ":" + join(sort(unique(prop("tags"))), "|") + " / v${formulaVersion}"`),
        formula('tokens', 'concat(split(upper(trim(prop("label"))), " "), prop("tags"))'),
        formula('weight', 'length(prop("label")) + length(prop("tags"))'),
      ];
    case 'dependency':
      // Seven requested targets contain a chain and a diamond, below the old
      // evaluator's depth and work limits. Both engines demand every target.
      return [
        input('a'), input('b'), input('c'),
        formula('base', 'prop("a") + prop("b")'),
        formula('left', 'prop("base") * 2'),
        formula('right', 'prop("base") + prop("c")'),
        formula('merged', 'prop("left") + prop("right")'),
        formula('chain1', 'prop("merged") + 1'),
        formula('chain2', 'prop("chain1") * 2'),
        formula('total', `prop("chain2") + prop("base") + ${bias}`),
      ];
    case 'business-long':
      // An invoice workflow uses the same cancellation/payment rules as the
      // production business fixtures. The long saved formula names its real
      // accounting steps; it is not a repeated expression padded for length.
      return [
        input('price'), input('quantity'), input('discount'), input('tax'),
        input('cancelled', FieldType.Checkbox), input('paid', FieldType.Checkbox),
        input('customer', FieldType.RichText),
        formula('total', 'if(prop("cancelled") or empty(prop("price")) or empty(prop("quantity")), 0, round(prop("price") * prop("quantity") * (1 - prop("discount")) * (1 + prop("tax")) * 100) / 100)'),
        formula('outstanding', `lets(
  unitPrice, prop("price"),
  units, prop("quantity"),
  discountRate, prop("discount"),
  taxRate, prop("tax"),
  invoiceAdjustment, ${bias},
  cancelled, prop("cancelled"),
  paid, prop("paid"),
  subtotal, unitPrice * units,
  discountAmount, subtotal * discountRate,
  taxableAmount, subtotal - discountAmount,
  taxAmount, taxableAmount * taxRate,
  invoiceAmount, round((taxableAmount + taxAmount + invoiceAdjustment) * 100) / 100,
  if(cancelled or empty(unitPrice) or empty(units), 0, if(paid, 0, invoiceAmount))
)`),
        formula('description', 'upper(trim(prop("customer"))) + " / " + if(prop("cancelled"), "Cancelled", if(prop("paid"), "Paid", "Outstanding")) + " / unit " + format(prop("price"))'),
      ];
  }
}

function randomGenerator(seed: number) {
  let state = seed >>> 0;

  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let next = state;

    next = Math.imul(next ^ (next >>> 15), next | 1);
    next ^= next + Math.imul(next ^ (next >>> 7), next | 61);
    return ((next ^ (next >>> 14)) >>> 0) / 4294967296;
  };
}

function modelFor(index: number, seed: number, random: () => number): RowModel {
  const base = 10 + Math.floor(random() * 90);

  return {
    index, base, a: base,
    b: 1 + Math.floor(random() * 20),
    c: Math.floor(random() * 30),
    label: `  Customer ${index} wave ${seed % 97}  `,
    tags: Array.from({ length: 1 + Math.floor(random() * 4) }, () => tagNames[Math.floor(random() * tagNames.length)]),
    price: base,
    quantity: 1 + Math.floor(random() * 10),
    discountEighths: Math.floor(random() * 3),
    taxSixteenths: Math.floor(random() * 3),
    cancelled: index % 11 === 0,
    paid: index % 3 === 0,
  };
}

function storedInputs(family: Family, row: RowModel): Record<string, string> {
  if (family === 'text-list') {
    return { label: row.label, tags: row.tags.map((name) => `tag-${tagNames.indexOf(name)}`).join(',') };
  }

  if (family === 'business-long') {
    return {
      price: String(row.price), quantity: String(row.quantity),
      discount: String(row.discountEighths / 8), tax: String(row.taxSixteenths / 16),
      cancelled: row.cancelled ? 'Yes' : 'No', paid: row.paid ? 'Yes' : 'No', customer: row.label,
    };
  }

  return { a: String(row.a), b: String(row.b), c: String(row.c) };
}

const numberValue = (value: number): FormulaValue => ({ type: 'number', value });
const textValue = (value: string): FormulaValue => ({ type: 'text', value });
const listValue = (items: string[]): FormulaValue => ({ type: 'list', items: items.map(textValue) });

/** Hand-authored models of the workloads, independent of either formula engine. */
function expectedValues(family: Family, row: RowModel, formulaVersion: number): FormulaValue[] {
  const bias = formulaVersion * 7;

  if (family === 'arithmetic') {
    return [row.a + row.b, row.a * row.b + row.c + bias, (row.a - row.b) / 2 + row.c].map(numberValue);
  }

  if (family === 'text-list') {
    return [
      textValue(`${row.label.trim().toLowerCase()}:${Array.from(new Set(row.tags)).sort().join('|')} / v${formulaVersion}`),
      listValue([...row.label.trim().toUpperCase().split(' '), ...row.tags]),
      numberValue(row.label.length + row.tags.length),
    ];
  }

  if (family === 'dependency') {
    const base = row.a + row.b;
    const left = base * 2;
    const right = base + row.c;
    const merged = left + right;
    const chain1 = merged + 1;
    const chain2 = chain1 * 2;

    return [base, left, right, merged, chain1, chain2, chain2 + base + bias].map(numberValue);
  }

  // Prices are integral; rates have denominators 8 and 16. Integer arithmetic
  // gives an independent cents oracle without floating-point rounding ties.
  const numerator = row.price * row.quantity * (8 - row.discountEighths) * (16 + row.taxSixteenths) * 100;
  const cents = Math.floor((numerator + 64) / 128);
  const amount = cents / 100;
  const total = row.cancelled ? 0 : amount;
  const outstanding = row.cancelled || row.paid ? 0 : (cents + bias * 100) / 100;
  const status = row.cancelled ? 'Cancelled' : row.paid ? 'Paid' : 'Outstanding';

  return [numberValue(total), numberValue(outstanding), textValue(`${row.label.trim().toUpperCase()} / ${status} / unit ${row.price}`)];
}

function expectedType(value: FormulaValue): FormulaType {
  if (value.type === 'list') return { list: 'text' };
  return value.type;
}

function expectedDisplay(value: FormulaValue): string {
  if (value.type === 'number' || value.type === 'text') return String(value.value);
  if (value.type === 'list') return value.items.map(expectedDisplay).join(', ');
  throw new Error(`No oracle for ${value.type}`);
}

export class BenchmarkDataset {
  readonly databaseDoc = new Y.Doc() as YDoc;
  readonly database = new Y.Map() as YDatabase;
  readonly fields = new Y.Map() as YDatabaseFields;
  readonly rows: Record<string, YDoc> = {};
  readonly rowValues: YDatabaseRow[] = [];
  readonly rowIds: string[] = [];
  readonly formulaIds: string[];
  readonly inputFieldIds: string[];
  readonly specs: FieldSpec[];
  private readonly models: RowModel[];
  private formulaVersion = 0;
  private editVersion = 0;

  constructor(readonly options: SetupOptions) {
    const random = randomGenerator(options.seed);

    this.specs = fieldsFor(options.family, 0);
    this.formulaIds = this.specs.filter((field) => field.type === FieldType.Formula).map((field) => field.id);
    this.inputFieldIds = this.specs.filter((field) => field.type !== FieldType.Formula).map((field) => field.id);
    this.models = Array.from({ length: options.rows }, (_, index) => modelFor(index, options.seed, random));
    this.databaseDoc.transact(() => {
      this.databaseDoc.getMap(E.data_section).set(E.database, this.database);
      this.database.set(K.id, 'benchmark-database');
      this.database.set(K.fields, this.fields);
      for (const spec of this.specs) {
        const field = new Y.Map() as YDatabaseField;

        this.fields.set(spec.id, field);
        field.set(K.id, spec.id);
        field.set(K.name, spec.id);
        field.set(K.type, spec.type);
        if (spec.options) {
          const typeOptions = new Y.Map();
          const option = new Y.Map();

          field.set(K.type_option, typeOptions);
          typeOptions.set(String(spec.type), option);
          Object.entries(spec.options).forEach(([key, value]) => option.set(key, value));
        }
      }
    });

    this.models.forEach((model, index) => {
      const id = `row-${String(index).padStart(5, '0')}`;
      const doc = new Y.Doc() as YDoc;
      const row = new Y.Map() as YDatabaseRow;
      const cells = new Y.Map() as YDatabaseCells;

      doc.transact(() => {
        doc.getMap(E.data_section).set(E.database_row, row);
        row.set(K.id, id);
        row.set(K.cells, cells);
        for (const [fieldId, data] of Object.entries(storedInputs(options.family, model))) {
          const cell = new Y.Map() as YDatabaseCell;

          cells.set(fieldId, cell);
          cell.set(K.field_type, this.fields.get(fieldId).get(K.type));
          cell.set(K.data, data);
        }
      });
      this.rows[id] = doc;
      this.rowIds.push(id);
      this.rowValues.push(row);
    });

    this.databaseDoc.transact(() => {
      const view = new Y.Map() as YDatabaseView;
      const views = new Y.Map() as YDatabaseViews;

      view.set(K.id, 'benchmark-view');
      view.set(K.row_orders, Y.Array.from(this.rowIds.map((id) => ({ id, height: 44 }))));
      views.set('benchmark-view', view);
      this.database.set(K.views, views);
    });
  }

  /** Prepare expected state outside the timer; the returned Yjs write is timed. */
  prepareMutation(operation: Operation, iteration: number): () => void {
    if (operation === 'edit-formula') {
      this.formulaVersion += 1;
      const nextSpecs = fieldsFor(this.options.family, this.formulaVersion);
      const next = nextSpecs.find((field) => field.type === FieldType.Formula && field.options?.expression !== this.specs.find((current) => current.id === field.id)?.options?.expression)!;
      const expression = String(next.options!.expression);
      const option = this.fields.get(next.id).get(K.type_option).get(String(FieldType.Formula));

      return () => this.databaseDoc.transact(() => option.set('expression', expression));
    }

    if (operation !== 'edit-cell' && operation !== 'edit-all') throw new Error(`Invalid mutation ${operation}`);
    this.editVersion += 1;
    const rowIndex = ((iteration % this.models.length) + this.models.length) % this.models.length;
    const selected = operation === 'edit-all' ? this.models : [this.models[rowIndex]];
    const changes = selected.map((model) => {
      const fieldId = this.options.family === 'text-list' ? 'label' : this.options.family === 'business-long' ? 'price' : 'a';

      if (fieldId === 'label') model.label = `  Customer ${model.index} wave ${this.options.seed % 97} batch ${this.editVersion}  `;
      else if (fieldId === 'price') model.price = model.base + this.editVersion;
      else model.a = model.base + this.editVersion;
      const value = fieldId === 'label' ? model.label : String(fieldId === 'price' ? model.price : model.a);
      const doc = this.rows[this.rowIds[model.index]];
      const cell = this.rowValues[model.index].get(K.cells).get(fieldId);

      return { doc, cell, value };
    });

    return () => changes.forEach(({ doc, cell, value }) => doc.transact(() => cell.set(K.data, value)));
  }

  expected(rowIndex: number): Array<Pick<FormulaCellResult, 'value' | 'resultType' | 'text'>> {
    return expectedValues(this.options.family, this.models[rowIndex], this.formulaVersion).map((value) => ({
      value, resultType: expectedType(value), text: expectedDisplay(value),
    }));
  }

  destroy() {
    Object.values(this.rows).forEach((doc) => doc.destroy());
    this.databaseDoc.destroy();
  }
}
