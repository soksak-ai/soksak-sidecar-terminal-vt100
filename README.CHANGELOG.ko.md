# VT100 적격성 변경 기록

## DEC Special Graphics

선 그리기 계약은 DEC Special Graphics 지정과 호출을 요구하며, fixture와 복구 로직은 변경하지 않았다.

이 owner는 `Cargo.toml`에 선언된 revision `5580fbb6dd389d18afbbd430fe3942867b02ae12`을 고정한다. 해당 revision은 필요한 문자 집합 동작을 구현하고 DEC 9 X10과 DEC 1001 highlight 입력을 서로 다른 상태로 노출한다. 일곱 fixture 적합성 suite는 7/7을 통과한다.
