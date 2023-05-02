import { engine } from '../dist/seedwing-policy-engine-component.js';
import { RuntimeValueObject,
	 Object,
	 ObjectValueString,
	 EvaluationResultOuter,
	 EvaluationResult,
         DataType} from '../dist/imports/policy-types';

import { inspect } from 'util';


console.log(`Seedwing Policy Engine version: ${engine.version()}`);

const policies: string[] = [];
const data: [string, DataType][] = [];
const policy = 'pattern dog = { name: string, trained: boolean }';
const name = "dog"
const input: RuntimeValueObject = {
  tag: 'object',
  val: [
      {key: "name", value: {tag: 'string', val: "goodboy"}},
      {key: "trained", value: {tag: 'boolean', val: true}},
  ]
};

const outer: EvaluationResultOuter = engine.eval(policies, data, policy, name, input);
const result = outer.evaluationResult;

console.log('EvaluationResult:');
console.log('input: ', inspect(result.input, {depth: 8}));
console.log('ty: ', inspect(result.ty, {depth: 8}));
console.log('rationale: ', inspect(result.rationale, {depth: 4}));
console.log('output: ', inspect(result.output, {depth: 4}));
//console.log('evaluation_result_map: ', inspect(outer.evaluationResultMap, {depth: 8}));
//console.log('pattern_map: ', inspect(outer.patternMap, {depth: 8}));
