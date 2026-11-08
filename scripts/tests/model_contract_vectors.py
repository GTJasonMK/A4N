"""Deterministic structural cases from production schemas, judged by two engines.

Business source/reference/state rules are exercised by the Rust and CLI suites;
these cases deliberately measure structural contracts rather than artistic output.
"""
from __future__ import annotations
import copy
import math
from typing import Any, Iterator


def resolve(schema: dict, root: dict) -> dict:
    if schema is True: return {}
    if schema is False: raise ValueError("false Schema 不能构造合法示例")
    seen = set()
    while "$ref" in schema:
        reference = schema["$ref"]
        if not reference.startswith("#/") or reference in seen:
            raise ValueError("合同只能使用无循环的内部引用")
        seen.add(reference)
        target = root
        for segment in reference[2:].split("/"):
            target = target[segment.replace("~1", "/").replace("~0", "~")]
        schema = merge_schema(target,{key:value for key,value in schema.items() if key != "$ref"})
    return schema


def sample(schema: dict, root: dict | None = None, depth: int = 0) -> Any:
    root = schema if root is None else root
    if depth > 32:
        raise ValueError("合同示例递归超过 32 层")
    schema = resolve(schema, root)
    if "const" in schema: return copy.deepcopy(schema["const"])
    if schema.get("enum"): return copy.deepcopy(schema["enum"][0])
    if schema.get("not")=={"type":"null"}: return "value"
    for alternatives in ("oneOf", "anyOf"):
        if alternatives in schema:
            return sample(schema[alternatives][0],root,depth+1)
    kind = schema.get("type")
    if isinstance(kind,list): kind = next((value for value in kind if value != "null"),"null")
    if kind == "object" or "properties" in schema:
        result = {key:sample(value,root,depth+1) for key,value in schema.get("properties",{}).items()}
        additional = schema.get("additionalProperties")
        if not result and isinstance(additional,dict) and additional:
            result["example"] = sample(additional,root,depth+1)
        for constraint in schema.get("allOf",[]):
            if "if" not in constraint: continue
            extra=constraint.get("then" if condition_matches(result,constraint["if"],root) else "else",{})
            for key,field in extra.get("properties",{}).items():
                if key in result or key in extra.get("required",[]):
                    child=merge_schema(resolve(schema.get("properties",{}).get(key,{}),root),field)
                    result[key]=sample(child,root,depth+1)
        return result
    if kind == "array":
        count = max(1,schema.get("minItems",0))
        if "maxItems" in schema: count = min(count,schema["maxItems"])
        values = [sample(schema.get("items",{}),root,depth+1) for _ in range(count)]
        if schema.get("uniqueItems") and count > 1:
            for index,value in enumerate(values):
                if isinstance(value,str): values[index] = value + str(index)
                elif type(value) in (int,float): values[index] = value + index
        return values
    if kind == "string":
        value = "value"
        pattern = schema.get("pattern","")
        if "[0-9" in pattern or "\\d" in pattern: value = "1"
        value = value.ljust(schema.get("minLength",0),"a")
        if "maxLength" in schema: value = value[:schema["maxLength"]]
        return value
    if kind in ("integer","number"):
        value = max(0,schema.get("minimum",0))
        if "exclusiveMinimum" in schema: value = max(value,math.floor(schema["exclusiveMinimum"])+1)
        return int(value) if kind == "integer" else value
    if kind == "boolean": return True
    return None


def matches_shape(value: Any, schema: dict, root: dict) -> bool:
    schema = resolve(schema,root)
    if "const" in schema: return value == schema["const"] and type(value) is type(schema["const"])
    if "enum" in schema: return value in schema["enum"]
    if "anyOf" in schema: return any(matches_shape(value,item,root) for item in schema["anyOf"])
    if "oneOf" in schema: return sum(matches_shape(value,item,root) for item in schema["oneOf"])==1
    kind = schema.get("type")
    if isinstance(kind,list): return any(matches_shape(value,{**schema,"type":item},root) for item in kind)
    matched={"object":isinstance(value,dict),"array":isinstance(value,list),"string":isinstance(value,str),"integer":type(value) is int,"number":type(value) in (int,float),"boolean":type(value) is bool,"null":value is None}.get(kind,True)
    if not matched: return False
    if isinstance(value,dict):
        if any(key not in value for key in schema.get("required",[])): return False
        return all(key not in value or matches_shape(value[key],field,root) for key,field in schema.get("properties",{}).items())
    return True


def allows_type(value: Any, schema: dict, root: dict) -> bool:
    schema = resolve(schema,root)
    for alternatives in ("anyOf","oneOf"):
        if alternatives in schema:
            return any(allows_type(value,item,root) for item in schema[alternatives])
    return matches_shape(value,{"type":schema.get("type")},root)


def merge_schema(base: dict, extra: dict) -> dict:
    result = copy.deepcopy(base)
    for key,value in extra.items():
        if key == "required": result[key] = list(dict.fromkeys([*result.get(key,[]),*value]))
        elif key == "properties":
            for name,field in value.items():
                result.setdefault(key,{})[name] = merge_schema(result.get(key,{}).get(name,{}),field)
        else: result[key] = copy.deepcopy(value)
    return result


def condition_matches(value: Any, condition: dict, root: dict) -> bool:
    condition = resolve(condition,root)
    if not matches_shape(value,condition,root): return False
    if isinstance(value,dict):
        if any(key not in value for key in condition.get("required",[])): return False
        return all(key not in value or condition_matches(value[key],field,root) for key,field in condition.get("properties",{}).items())
    return True


def nodes(schema: dict, value: Any, root: dict, path: tuple = ()) -> Iterator[tuple]:
    schema = resolve(schema,root)
    for alternative in ("oneOf","anyOf"):
        if alternative in schema:
            selected = next((item for item in schema[alternative] if matches_shape(value,item,root)),None)
            if selected is None: raise ValueError("示例没有匹配合同分支")
            schema = {**schema,**resolve(selected,root)}
            break
    for constraint in schema.get("allOf",[]):
        constraint = resolve(constraint,root)
        if "if" in constraint:
            selected = constraint.get("then" if condition_matches(value,constraint["if"],root) else "else",{})
        else: selected = constraint
        schema = merge_schema(schema,selected)
    yield path,schema,value
    if isinstance(value,dict):
        for key,item in value.items():
            child = schema.get("properties",{}).get(key,schema.get("additionalProperties",{}))
            if isinstance(child,dict): yield from nodes(child,item,root,(*path,key))
    elif isinstance(value,list) and value:
        child = schema.get("items",{})
        if isinstance(child,dict): yield from nodes(child,value[0],root,(*path,0))


def replace(value: Any, path: tuple, replacement: Any, remove: bool = False) -> Any:
    result = copy.deepcopy(value)
    if not path: return copy.deepcopy(replacement)
    cursor = result
    for key in path[:-1]: cursor = cursor[key]
    if remove: del cursor[path[-1]]
    else: cursor[path[-1]] = copy.deepcopy(replacement)
    return result


def vectors(call_id: str, schema: dict, example: Any | None = None) -> list[dict]:
    example = sample(schema) if example is None else copy.deepcopy(example)
    result = [{"case_id":f"{call_id}/contract/valid","call_id":call_id,"schema":schema,"value":example,"expected":True,"rule":"valid"}]
    def add(path,rule,replacement,expected=False,remove=False):
        pointer = "/" + "/".join(str(key).replace("~","~0").replace("/","~1") for key in path)
        result.append({"case_id":f"{call_id}/contract/{pointer}:{rule}","call_id":call_id,"schema":schema,"value":replace(example,path,replacement,remove),"expected":expected,"rule":rule,"path":pointer})
    for path,field,value in nodes(schema,example,schema):
        if "type" in field:
            for name,wrong in [("object",{}),("array",[]),("string","wrong"),("integer",3),("boolean",False),("null",None)]:
                if not allows_type(wrong,field,schema): add(path,"type-"+name,wrong)
        if "const" in field: add(path,"const",str(field["const"])+"_wrong")
        if "enum" in field: add(path,"enum","__invalid_enum__")
        if field.get("not")=={"type":"null"}: add(path,"non-null",None)
        if isinstance(value,dict):
            for key in field.get("required",[]):
                if key not in value: raise ValueError(f"{call_id} 的示例缺少必填 {path}/{key}")
                add((*path,key),"missing",None,remove=True)
            for key in value:
                if key not in field.get("required",[]) and field.get("minProperties",0) < len(value):
                    add((*path,key),"optional-absent",None,True,True)
            if field.get("additionalProperties") is False:
                add(path,"unknown-field",{**value,"__unexpected":None})
            if field.get("required") or field.get("minProperties",0)>0: add(path,"empty-object",{})
        if isinstance(value,str):
            if field.get("minLength",0)>0: add(path,"empty-string","")
            if field.get("pattern") == r"\S": add(path,"whitespace"," \n\t ")
            if "maxLength" in field: add(path,"max-length","a"*(field["maxLength"]+1))
        if type(value) in (int,float):
            if "minimum" in field: add(path,"minimum",field["minimum"]-max(1,abs(field["minimum"])//1000))
            if "exclusiveMinimum" in field: add(path,"exclusive-minimum",field["exclusiveMinimum"])
            if "maximum" in field: add(path,"maximum",field["maximum"]+max(1,abs(field["maximum"])//1000))
            if "exclusiveMaximum" in field: add(path,"exclusive-maximum",field["exclusiveMaximum"])
            if field.get("type") == "integer": add(path,"fraction",1.25)
        if isinstance(value,list):
            if field.get("minItems",0)>0: add(path,"empty-array",[])
            if field.get("uniqueItems") and value: add(path,"duplicate-item",[value[0],value[0]])
            if "maxItems" in field and value: add(path,"max-items",[value[0]]*(field["maxItems"]+1))
    case_ids = [case["case_id"] for case in result]
    if len(case_ids) != len(set(case_ids)): raise ValueError("合同变异用例 ID 重复")
    return result


def judge(expected: list[dict], actual: list[dict]) -> None:
    observed = {case["case_id"]:case for case in actual}
    if len(observed) != len(actual) or set(observed) != {case["case_id"] for case in expected}:
        raise ValueError("校验器实际执行用例与要求不一致")
    for case in expected:
        if observed[case["case_id"]].get("accepted") is not case["expected"]:
            raise ValueError(f"合同测试未捕获预期结果：{case['case_id']}; expected={case['expected']}; actual={observed[case['case_id']]}")
