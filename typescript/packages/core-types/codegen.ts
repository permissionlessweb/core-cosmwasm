import codegen from '@cosmwasm/ts-codegen'

codegen({
  contracts: [
    {
      name: 'FairBurn',
      dir: '../../../contracts/fair-burn/schema',
    },
    {
      name: 'ResidualRegistry',
      dir: '../../../contracts/residual-registry/schema',
    },
  ],
  outPath: './src/',

  options: {
    bundle: {
      bundleFile: 'index.ts',
      scope: 'contracts',
    },
    types: {
      enabled: true,
    },
    client: {
      enabled: true,
    },
    reactQuery: {
      enabled: true,
      optionalClient: true,
      version: 'v3',
      mutations: true,
      queryKeys: true,
      queryFactory: true,
    },
    recoil: {
      enabled: false,
    },
    messageComposer: {
      enabled: true,
    },
  },
}).then(() => {
  console.log('✨ all done!')
})
